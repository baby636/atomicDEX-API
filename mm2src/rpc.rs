/******************************************************************************
 * Copyright © 2014-2018 The SuperNET Developers.                             *
 *                                                                            *
 * See the AUTHORS, DEVELOPER-AGREEMENT and LICENSE files at                  *
 * the top-level directory of this distribution for the individual copyright  *
 * holder information and the developer policies on copyright and licensing.  *
 *                                                                            *
 * Unless otherwise agreed in a custom licensing agreement, no part of the    *
 * SuperNET software, including this file may be copied, modified, propagated *
 * or distributed except according to the terms contained in the LICENSE file *
 *                                                                            *
 * Removal or modification of this copyright notice is prohibited.            *
 *                                                                            *
 ******************************************************************************/
//
//  rpc.rs
//
//  Copyright © 2014-2018 SuperNET. All rights reserved.
//
use futures::{self, future, Future};
use futures_cpupool::CpuPool;
use gstuff;
use helpers::{free_c_ptr, lp, MmArc, CORE};
use hyper::{Response, Request, Body, Method};
use hyper::server::conn::Http;
use hyper::rt::{Stream};
use hyper::service::Service;
use hyper::header::{HeaderValue, CONTENT_TYPE};
use serde::Serialize;
use serde_json::{self as json, Value as Json};
use std::ffi::{CStr, CString};
use std::net::{SocketAddr};
use std::ptr::null_mut;
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;
use super::CJSON;
use tokio_core::net::TcpListener;
use hex;

/// Returns a JSON error HyRes on a failure.
macro_rules! try_h {
    ($e: expr) => {
        match $e {
            Ok (ok) => ok,
            Err (err) => {return err_response (500, &ERRL! ("{}", err))}
        }
    }
}

mod commands;
use self::commands::*;

lazy_static! {
    /// Shared HTTP server.
    pub static ref HTTP: Http = Http::new();
    /// Shared CPU pool to run intensive/sleeping requests on separate thread
    pub static ref CPUPOOL: CpuPool = CpuPool::new(8);
}

const STATS_VALID_METHODS : &[&str] = &[
    "psock", "ticker", "balances", "getprice", "notify", "getpeers", "orderbook",
    "statsdisp", "fundvalue", "help", "getcoins", "pricearray", "balance", "tradesarray"
];

fn lp_valid_remote_method(method: &str) -> bool {
    STATS_VALID_METHODS.iter().position(|&s| s == method).is_some()
}

const PORTED_FAST_METHODS : &[Option<&str>] = &[Some("version"), Some("help"),
    Some("eth_gas_price"), Some("autoprice"), Some("mpnet")];

fn is_fast_ported(method: Option<&str>) -> bool {
    PORTED_FAST_METHODS.iter().position(|&s| s == method).is_some()
}

#[allow(unused_macros)]
macro_rules! unwrap_or_err_response {
    ($e:expr, $($args:tt)*) => {
        match $e {
            Ok (ok) => ok,
            Err (err) => {return err_response (500, &ERRL! ("{}", err))}
        }
    }
}

macro_rules! unwrap_or_err_msg {
    ($e:expr, $($args:tt)*) => {
        match $e {
            Ok(ok) => ok,
            Err(_e) => {
                return Ok(err_to_json_string($($args)*))
            }
        }
    }
}

#[derive(Serialize)]
struct ErrResponse {
    error: String,
}

#[derive(Serialize)]
struct SuccessResponse<T> {
    result: T,
}

fn serialize_result<T> (result: T) -> Result<String, String>
    where T: Serialize {
    json::to_string(&SuccessResponse {
        result
    }).map_err(|e| err_to_json_string(&ERRL!("{}", e)))
}

struct RpcService {
    /// Allows us to get the `MmCtx` if it is still around.
    ctx_h: u32,
    /// The IP and port from whence the request is coming from.
    remote_addr: SocketAddr,
}

fn rpc_process_json(ctx: MmArc, remote_addr: SocketAddr, json: Json)
                        -> Result<String, String> {
    if !remote_addr.ip().is_loopback() && !lp_valid_remote_method(json["method"].as_str().unwrap()) {
        return Ok(err_to_json_string("Selected method can be called from localhost only!"));
    }

    // It's not required to authenticate to call remote method
    if !lp_valid_remote_method(json["method"].as_str().unwrap()) {
        if !json["userpass"].is_string() {
            return Ok(err_to_json_string("Userpass is not set!"));
        }

        let userpass = unsafe { CStr::from_ptr(lp::G.USERPASS.as_ptr()).to_str().unwrap() };
        let pass_hash = hex::encode(unsafe { lp::G.LP_passhash.bytes });

        if json["userpass"].as_str() != Some(userpass) && json["userpass"].as_str() != Some(&pass_hash) {
            return Ok(err_to_json_string("Userpass is invalid!"));
        }
    }

    let c_json = unwrap_or_err_msg!(CJSON::from_str(&json.to_string()),
                                        "Couldn't parse request body as json");

    if is_fast_ported(json["method"].as_str()) {
        return call_method(ctx, json, c_json);
    }

    if !json["queueid"].is_null() {
        if json["queueid"].is_u64() {
            if unsafe { lp::IPC_ENDPOINT == -1 } {
                return Ok(err_to_json_string("Can't queue the command when ws endpoint is disabled!"));
            } else if !remote_addr.ip().is_loopback() {
                return Ok(err_to_json_string("Can queue the command from localhost only!"));
            } else {
                let json_str = json.to_string();
                let c_json_ptr = unwrap_or_err_msg!(CString::new(json_str), "Error occurred");
                unsafe {
                    lp::LP_queuecommand(null_mut(),
                                        c_json_ptr.as_ptr() as *mut c_char,
                                        lp::IPC_ENDPOINT,
                                        1,
                                        json["queueid"].as_u64().unwrap() as u32
                    );
                }
                return Ok(r#"{"result":"success","status":"queued"}"#.to_string());
            }
        } else {
            return Ok(err_to_json_string("queueid must be unsigned integer!"));
        }
    }

    let my_ip_ptr = unwrap_or_err_msg!(CString::new(format!("{}", ctx.rpc_ip_port.ip())),
                                        "Error occurred");
    let remote_ip_ptr = unwrap_or_err_msg!(CString::new(format!("{}", remote_addr.ip())),
                                        "Error occurred");
    let stats_result = unsafe {
        lp::stats_JSON(
            ctx.btc_ctx() as *mut c_void,
            0,
            my_ip_ptr.as_ptr() as *mut c_char,
            -1,
            c_json.0,
            remote_ip_ptr.as_ptr() as *mut c_char,
            ctx.rpc_ip_port.port()
        )
    };

    if !stats_result.is_null() {
        let res_str = unsafe {
            unwrap_or_err_msg!(CStr::from_ptr(stats_result).to_str(),
            "Request execution result is empty")
        };
        let res_str = String::from (res_str);
        free_c_ptr(stats_result as *mut c_void);
        Ok(res_str)
    } else {
        Ok(err_to_json_string("Request execution result is empty"))
    }
}

type HyRes = Box<Future<Item=Response<Body>, Error=String> + Send>;

fn rpc_response<T>(status: u16, body: T) -> HyRes where Body: From<T> {
    Box::new (
        match Response::builder()
            .status(status)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(Body::from(body)) {
                Ok (r) => future::ok::<Response<Body>, String> (r),
                Err (err) => future::err::<Response<Body>, String> (ERRL! ("{}", err))
            }
    )
}

fn err_to_json_string(err: &str) -> String {
    let err = ErrResponse {
        error: err.to_owned(),
    };
    json::to_string(&err).unwrap()
}

fn err_response(status: u16, msg: &str) -> HyRes {
    rpc_response(status, err_to_json_string(msg))
}

/// The outer dispatcher, with full control over the HTTP result and the way we run the `Future` producing it.
fn dispatcher (req: Json, remote_addr: SocketAddr, ctx_h: u32) -> HyRes {
    lazy_static! {static ref SINGLE_THREADED_C_LOCK: Mutex<()> = Mutex::new(());}

    let method = req["method"].as_str().map (|s| s.to_string());
    let method = match method {Some (ref s) => Some (&s[..]), None => None};
    match method {
        Some ("stop") => stop (ctx_h),
        None => err_response (400, "Method is not set!"),
        _ => {  // Evoke the old C code.
            let cpu_pool_fut = CPUPOOL.spawn_fn(move || {
                let ctx = try_s! (MmArc::from_ffi_handler (ctx_h));
                // Emulates the single-threaded execution of the old C code.
                let _lock = SINGLE_THREADED_C_LOCK.lock();
                rpc_process_json (ctx, remote_addr, req)
            });
            rpc_response (200, Body::wrap_stream (cpu_pool_fut.into_stream()))
        }
    }
}

/// The inner dispatcher, invoked from `rpc_process_json` for some of the fast and protected RPC methods.
fn call_method(ctx: MmArc, json: Json, c_json: CJSON) -> Result<String, String> {
    let result = match json["method"].as_str() {
        Some("version") => version(),
        Some("help") => help(),
        Some("eth_gas_price") => eth_gas_price(),
        Some("autoprice") => auto_price(ctx, &json, c_json),
        Some("mpnet") => mpnet(&json),
        _ => return Err(err_to_json_string("Invalid method"))
    };
    match result {
        Ok(success) => serialize_result(success),
        Err(e) => Err(err_to_json_string(e))
    }
}

impl Service for RpcService {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = String;
    type Future = HyRes;

    fn call(&mut self, request: Request<Body>) -> HyRes {
        if request.method() != Method::POST {
            return err_response (400, "Only POST requests are supported!")
        }
        let body_f = request.into_body().concat2();

        let remote_addr = self.remote_addr.clone();
        let ctx_h = self.ctx_h;

        let f = body_f.then (move |req| -> HyRes {
            let req = try_h! (req);
            let req: Json = try_h! (json::from_slice (&req));
            dispatcher (req, remote_addr, ctx_h)
        });

        Box::new (f)
    }
}

pub extern fn spawn_rpc(ctx_h: u32) {
    // NB: We need to manually handle the incoming connections in order to get the remote IP address,
    // cf. https://github.com/hyperium/hyper/issues/1410#issuecomment-419510220.
    // Although if the ability to access the remote IP address is solved by the Hyper in the future
    // then we might want to refactor into starting it ideomatically in order to benefit from a more graceful shutdown,
    // cf. https://github.com/hyperium/hyper/pull/1640.

    let ctx = unwrap! (MmArc::from_ffi_handler (ctx_h), "No context");

    let listener = unwrap! (TcpListener::bind2 (&ctx.rpc_ip_port), "Can't bind on {}", ctx.rpc_ip_port);

    let server = listener
        .incoming()
        .for_each(move |(socket, _my_sock)| {
            let remote_addr = match socket.peer_addr() {
                Ok (addr) => addr,
                Err (err) => {
                    eprintln! ("spawn_rpc] No peer_addr: {}", err);
                    return Ok(())
                }
            };

            CORE.spawn(move |_|
                HTTP.serve_connection(
                    socket,
                    RpcService {
                        ctx_h,
                        remote_addr
                    },
                )
                .map(|_| ())
                .map_err (|err| eprintln! ("spawn_rpc] HTTP error: {}", err))
            );
            Ok(())
        })
        .map_err (|err| eprintln! ("spawn_rpc] accept error: {}", err));

    // Finish the server `Future` when `shutdown_rx` fires.

    let (shutdown_tx, shutdown_rx) = futures::sync::oneshot::channel::<()>();
    let server = server.select2 (shutdown_rx) .then (|_| Ok(()));
    let mut shutdown_tx = Some (shutdown_tx);
    ctx.on_stop (Box::new (move || {
        if let Some (shutdown_tx) = shutdown_tx.take() {
            println! ("rpc] on_stop, firing shutdown_tx!");
            if let Err (_) = shutdown_tx.send(()) {ERR! ("shutdown_tx already closed")} else {Ok(())}
        } else {ERR! ("on_stop callback called twice!")}
    }));

    CORE.spawn(move |_| {
        ctx.log.rawln (
            format!(">>>>>>>>>> DEX stats {}:{} DEX stats API enabled at unixtime.{} <<<<<<<<<",
                    ctx.rpc_ip_port.ip(),
                    ctx.rpc_ip_port.port(),
                    gstuff::now_ms() / 1000
        ));
        server
    });
}
