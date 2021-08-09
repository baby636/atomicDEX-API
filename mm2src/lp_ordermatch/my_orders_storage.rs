use crate::mm2::lp_ordermatch::{MakerOrder, MakerOrderCancellationReason, MyOrdersFilter, Order,
                                RecentOrdersSelectResult, TakerOrder, TakerOrderCancellationReason};
use async_trait::async_trait;
use common::mm_ctx::MmArc;
use common::mm_error::prelude::*;
use common::{BoxFut, PagingOptions};
use derive_more::Display;
use futures::{FutureExt, TryFutureExt};
#[cfg(test)] use mocktopus::macros::*;
use uuid::Uuid;

pub type MyOrdersResult<T> = Result<T, MmError<MyOrdersError>>;

#[derive(Display)]
pub enum MyOrdersError {
    #[display(fmt = "Order with uuid {} is not found", uuid)]
    NoSuchOrder { uuid: Uuid },
    #[display(fmt = "Error uploading the a swap: {}", _0)]
    ErrorUploading(String),
    #[display(fmt = "Error loading a swap: {}", _0)]
    ErrorLoading(String),
    #[display(fmt = "Error deserializing a swap: {}", _0)]
    ErrorDeserializing(String),
    #[display(fmt = "Error serializing a swap: {}", _0)]
    ErrorSerializing(String),
    #[allow(dead_code)]
    #[display(fmt = "Internal error: {}", _0)]
    InternalError(String),
}

pub async fn save_my_new_maker_order(ctx: MmArc, order: &MakerOrder) {
    let storage = MyOrdersStorage::new(ctx);
    error_if!(storage.save_new_active_maker_order(order).await);

    if order.save_in_history {
        error_if!(storage.save_maker_order_in_filtering_history(order).await);
    }
}

pub async fn save_my_new_taker_order(ctx: MmArc, order: &TakerOrder) {
    let storage = MyOrdersStorage::new(ctx);
    error_if!(storage.save_new_active_taker_order(order).await);

    if order.save_in_history {
        error_if!(storage.save_taker_order_in_filtering_history(order).await);
    }
}

pub async fn save_maker_order_on_update(ctx: MmArc, order: &MakerOrder) {
    let storage = MyOrdersStorage::new(ctx);
    error_if!(storage.update_active_maker_order(order).await);

    if order.save_in_history {
        error_if!(storage.update_maker_order_in_filtering_history(order).await);
    }
}

#[cfg_attr(test, mockable)]
pub fn delete_my_taker_order(ctx: MmArc, order: TakerOrder, reason: TakerOrderCancellationReason) -> BoxFut<(), ()> {
    let fut = async move {
        let uuid = order.request.uuid;
        let save_in_history = order.save_in_history;

        let storage = MyOrdersStorage::new(ctx);
        error_if!(storage.delete_active_taker_order(uuid).await);

        match reason {
            TakerOrderCancellationReason::ToMaker => (),
            _ => {
                if save_in_history {
                    error_if!(storage.save_order_in_history(&Order::Taker(order)).await);
                }
            },
        }

        if save_in_history {
            error_if!(
                storage
                    .update_order_status_in_filtering_history(uuid, reason.to_string())
                    .await
            );
        }
        Ok(())
    };
    Box::new(fut.boxed().compat())
}

#[cfg_attr(test, mockable)]
pub fn delete_my_maker_order(ctx: MmArc, order: MakerOrder, reason: MakerOrderCancellationReason) -> BoxFut<(), ()> {
    let fut = async move {
        let uuid = order.uuid;
        let save_in_history = order.save_in_history;

        let storage = MyOrdersStorage::new(ctx);
        error_if!(storage.delete_active_maker_order(uuid).await);

        if save_in_history {
            error_if!(storage.save_order_in_history(&Order::Maker(order.clone())).await);
            error_if!(
                storage
                    .update_order_status_in_filtering_history(uuid, reason.to_string())
                    .await
            );
        }
        Ok(())
    };
    Box::new(fut.boxed().compat())
}

#[derive(Clone)]
pub struct MyOrdersStorage {
    ctx: MmArc,
}

impl MyOrdersStorage {
    pub fn new(ctx: MmArc) -> MyOrdersStorage { MyOrdersStorage { ctx } }
}

#[async_trait]
pub trait MyActiveOrders {
    async fn load_active_maker_orders(&self) -> MyOrdersResult<Vec<MakerOrder>>;

    async fn load_active_taker_orders(&self) -> MyOrdersResult<Vec<TakerOrder>>;

    async fn save_new_active_order(&self, order: &Order) -> MyOrdersResult<()> {
        match order {
            Order::Maker(maker) => self.save_new_active_maker_order(maker).await,
            Order::Taker(taker) => self.save_new_active_taker_order(taker).await,
        }
    }

    async fn save_new_active_maker_order(&self, order: &MakerOrder) -> MyOrdersResult<()>;

    async fn save_new_active_taker_order(&self, order: &TakerOrder) -> MyOrdersResult<()>;

    async fn delete_active_maker_order(&self, uuid: Uuid) -> MyOrdersResult<()>;

    async fn delete_active_taker_order(&self, uuid: Uuid) -> MyOrdersResult<()>;

    async fn update_active_maker_order(&self, order: &MakerOrder) -> MyOrdersResult<()>;

    async fn update_active_taker_order(&self, order: &TakerOrder) -> MyOrdersResult<()>;
}

#[async_trait]
pub trait MyOrdersHistory {
    async fn save_order_in_history(&self, order: &Order) -> MyOrdersResult<()>;

    async fn load_order_from_history(&self, uuid: Uuid) -> MyOrdersResult<Order>;
}

#[async_trait]
pub trait MyOrdersFilteringHistory {
    async fn select_orders_by_filter(
        &self,
        filter: &MyOrdersFilter,
        paging_options: Option<&PagingOptions>,
    ) -> MyOrdersResult<RecentOrdersSelectResult>;

    async fn select_order_status(&self, uuid: Uuid) -> MyOrdersResult<String>;

    async fn save_order_in_filtering_history(&self, order: &Order) -> MyOrdersResult<()> {
        match order {
            Order::Maker(maker) => self.save_maker_order_in_filtering_history(maker).await,
            Order::Taker(taker) => self.save_taker_order_in_filtering_history(taker).await,
        }
    }

    async fn save_maker_order_in_filtering_history(&self, order: &MakerOrder) -> MyOrdersResult<()>;

    async fn save_taker_order_in_filtering_history(&self, order: &TakerOrder) -> MyOrdersResult<()>;

    async fn update_maker_order_in_filtering_history(&self, order: &MakerOrder) -> MyOrdersResult<()>;

    async fn update_order_status_in_filtering_history(&self, uuid: Uuid, status: String) -> MyOrdersResult<()>;

    async fn update_was_taker_in_filtering_history(&self, uuid: Uuid) -> MyOrdersResult<()>;
}

#[cfg(not(target_arch = "wasm32"))]
mod native_impl {
    use super::*;
    use crate::mm2::database::my_orders::{insert_maker_order, insert_taker_order, select_orders_by_filter,
                                          select_status_by_uuid, update_maker_order, update_order_status,
                                          update_was_taker};
    use crate::mm2::lp_ordermatch::{my_maker_order_file_path, my_maker_orders_dir, my_order_history_file_path,
                                    my_taker_order_file_path, my_taker_orders_dir};
    use common::fs::{read_dir_json, read_json, remove_file_async, write_json, FsJsonError};

    impl From<FsJsonError> for MyOrdersError {
        fn from(fs: FsJsonError) -> Self {
            match fs {
                FsJsonError::IoReading(reading) => MyOrdersError::ErrorLoading(reading.to_string()),
                FsJsonError::IoWriting(writing) => MyOrdersError::ErrorUploading(writing.to_string()),
                FsJsonError::Serializing(serializing) => MyOrdersError::ErrorSerializing(serializing.to_string()),
                FsJsonError::Deserializing(deserializing) => {
                    MyOrdersError::ErrorDeserializing(deserializing.to_string())
                },
            }
        }
    }

    #[async_trait]
    impl MyActiveOrders for MyOrdersStorage {
        async fn load_active_maker_orders(&self) -> MyOrdersResult<Vec<MakerOrder>> {
            let dir_path = my_maker_orders_dir(&self.ctx);
            Ok(read_dir_json(&dir_path).await?)
        }

        async fn load_active_taker_orders(&self) -> MyOrdersResult<Vec<TakerOrder>> {
            let dir_path = my_taker_orders_dir(&self.ctx);
            Ok(read_dir_json(&dir_path).await?)
        }

        async fn save_new_active_maker_order(&self, order: &MakerOrder) -> MyOrdersResult<()> {
            let path = my_maker_order_file_path(&self.ctx, &order.uuid);
            write_json(order, &path).await?;
            Ok(())
        }

        async fn save_new_active_taker_order(&self, order: &TakerOrder) -> MyOrdersResult<()> {
            let path = my_taker_order_file_path(&self.ctx, &order.request.uuid);
            write_json(order, &path).await?;
            Ok(())
        }

        async fn delete_active_maker_order(&self, uuid: Uuid) -> MyOrdersResult<()> {
            let path = my_maker_order_file_path(&self.ctx, &uuid);
            remove_file_async(&path)
                .await
                .mm_err(|e| MyOrdersError::ErrorUploading(e.to_string()))?;
            Ok(())
        }

        async fn delete_active_taker_order(&self, uuid: Uuid) -> MyOrdersResult<()> {
            let path = my_taker_order_file_path(&self.ctx, &uuid);
            remove_file_async(&path)
                .await
                .mm_err(|e| MyOrdersError::ErrorUploading(e.to_string()))?;
            Ok(())
        }

        async fn update_active_maker_order(&self, order: &MakerOrder) -> MyOrdersResult<()> {
            self.save_new_active_maker_order(order).await
        }

        async fn update_active_taker_order(&self, order: &TakerOrder) -> MyOrdersResult<()> {
            self.save_new_active_taker_order(order).await
        }
    }

    #[async_trait]
    impl MyOrdersHistory for MyOrdersStorage {
        async fn save_order_in_history(&self, order: &Order) -> MyOrdersResult<()> {
            let uuid = match order {
                Order::Maker(maker) => &maker.uuid,
                Order::Taker(taker) => &taker.request.uuid,
            };
            let path = my_order_history_file_path(&self.ctx, uuid);
            write_json(order, &path).await?;
            Ok(())
        }

        async fn load_order_from_history(&self, uuid: Uuid) -> MyOrdersResult<Order> {
            let path = my_order_history_file_path(&self.ctx, &uuid);
            read_json(&path)
                .await?
                .or_mm_err(|| MyOrdersError::NoSuchOrder { uuid })
        }
    }

    #[async_trait]
    impl MyOrdersFilteringHistory for MyOrdersStorage {
        async fn select_orders_by_filter(
            &self,
            filter: &MyOrdersFilter,
            paging_options: Option<&PagingOptions>,
        ) -> MyOrdersResult<RecentOrdersSelectResult> {
            select_orders_by_filter(&self.ctx.sqlite_connection(), filter, paging_options)
                .map_to_mm(|e| MyOrdersError::ErrorLoading(e.to_string()))
        }

        async fn select_order_status(&self, uuid: Uuid) -> MyOrdersResult<String> {
            select_status_by_uuid(&self.ctx.sqlite_connection(), &uuid)
                .map_to_mm(|e| MyOrdersError::ErrorLoading(e.to_string()))
        }

        async fn save_maker_order_in_filtering_history(&self, order: &MakerOrder) -> MyOrdersResult<()> {
            insert_maker_order(&self.ctx, order.uuid, order).map_to_mm(|e| MyOrdersError::ErrorUploading(e.to_string()))
        }

        async fn save_taker_order_in_filtering_history(&self, order: &TakerOrder) -> MyOrdersResult<()> {
            insert_taker_order(&self.ctx, order.request.uuid, order)
                .map_to_mm(|e| MyOrdersError::ErrorUploading(e.to_string()))
        }

        async fn update_maker_order_in_filtering_history(&self, order: &MakerOrder) -> MyOrdersResult<()> {
            update_maker_order(&self.ctx, order.uuid, order).map_to_mm(|e| MyOrdersError::ErrorUploading(e.to_string()))
        }

        async fn update_order_status_in_filtering_history(&self, uuid: Uuid, status: String) -> MyOrdersResult<()> {
            update_order_status(&self.ctx, uuid, status).map_to_mm(|e| MyOrdersError::ErrorUploading(e.to_string()))
        }

        async fn update_was_taker_in_filtering_history(&self, uuid: Uuid) -> MyOrdersResult<()> {
            update_was_taker(&self.ctx, uuid).map_to_mm(|e| MyOrdersError::ErrorUploading(e.to_string()))
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
    use super::*;
    use common::log::warn;

    #[async_trait]
    impl MyActiveOrders for MyOrdersStorage {
        async fn load_active_maker_orders(&self) -> MyOrdersResult<Vec<MakerOrder>> {
            warn!("'load_active_maker_orders' not supported in WASM yet");
            MmError::err(MyOrdersError::InternalError(
                "'load_active_maker_orders' not supported in WASM".to_owned(),
            ))
        }

        async fn load_active_taker_orders(&self) -> MyOrdersResult<Vec<TakerOrder>> {
            warn!("'load_active_taker_orders' not supported in WASM yet");
            MmError::err(MyOrdersError::InternalError(
                "'load_active_taker_orders' not supported in WASM".to_owned(),
            ))
        }

        async fn save_new_active_maker_order(&self, _order: &MakerOrder) -> MyOrdersResult<()> {
            warn!("'save_new_active_maker_order' not supported in WASM yet");
            Ok(())
        }

        async fn save_new_active_taker_order(&self, _order: &TakerOrder) -> MyOrdersResult<()> {
            warn!("'save_new_active_taker_order' not supported in WASM yet");
            Ok(())
        }

        async fn delete_active_maker_order(&self, _uuid: Uuid) -> MyOrdersResult<()> {
            warn!("'delete_active_maker_order' not supported in WASM yet");
            Ok(())
        }

        async fn delete_active_taker_order(&self, _uuid: Uuid) -> MyOrdersResult<()> {
            warn!("'delete_active_taker_order' not supported in WASM yet");
            Ok(())
        }

        async fn update_active_maker_order(&self, _order: &MakerOrder) -> MyOrdersResult<()> {
            warn!("'update_active_maker_order' not supported in WASM yet");
            Ok(())
        }

        async fn update_active_taker_order(&self, _order: &TakerOrder) -> MyOrdersResult<()> {
            warn!("'update_active_taker_order' not supported in WASM yet");
            Ok(())
        }
    }

    #[async_trait]
    impl MyOrdersHistory for MyOrdersStorage {
        async fn save_order_in_history(&self, _order: &Order) -> MyOrdersResult<()> {
            warn!("'save_order_in_history' not supported in WASM yet");
            Ok(())
        }

        async fn load_order_from_history(&self, _uuid: Uuid) -> MyOrdersResult<Order> {
            warn!("'load_order_from_history' not supported in WASM yet");
            MmError::err(MyOrdersError::InternalError(
                "'load_order_from_history' not supported in WASM".to_owned(),
            ))
        }
    }

    #[async_trait]
    impl MyOrdersFilteringHistory for MyOrdersStorage {
        async fn select_orders_by_filter(
            &self,
            _filter: &MyOrdersFilter,
            _paging_options: Option<&PagingOptions>,
        ) -> MyOrdersResult<RecentOrdersSelectResult> {
            warn!("'select_orders_by_filter' not supported in WASM yet");
            MmError::err(MyOrdersError::InternalError(
                "'select_orders_by_filter' not supported in WASM".to_owned(),
            ))
        }

        async fn select_order_status(&self, _uuid: Uuid) -> MyOrdersResult<String> {
            warn!("'select_order_status' not supported in WASM yet");
            MmError::err(MyOrdersError::InternalError(
                "'select_order_status' not supported in WASM".to_owned(),
            ))
        }

        async fn save_maker_order_in_filtering_history(&self, _order: &MakerOrder) -> MyOrdersResult<()> {
            warn!("'save_maker_order_in_filtering_history' not supported in WASM yet");
            Ok(())
        }

        async fn save_taker_order_in_filtering_history(&self, _order: &TakerOrder) -> MyOrdersResult<()> {
            warn!("'save_taker_order_in_filtering_history' not supported in WASM yet");
            Ok(())
        }

        async fn update_maker_order_in_filtering_history(&self, _order: &MakerOrder) -> MyOrdersResult<()> {
            warn!("'update_maker_order_in_filtering_history' not supported in WASM yet");
            Ok(())
        }

        async fn update_order_status_in_filtering_history(&self, _uuid: Uuid, _status: String) -> MyOrdersResult<()> {
            warn!("'update_order_status_in_filtering_history' not supported in WASM yet");
            Ok(())
        }

        async fn update_was_taker_in_filtering_history(&self, _uuid: Uuid) -> MyOrdersResult<()> {
            warn!("'update_was_taker_in_filtering_history' not supported in WASM yet");
            Ok(())
        }
    }
}
