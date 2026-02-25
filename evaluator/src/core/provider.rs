use analyzer::analysis::Property;
use core::future::Future;

use super::errors::ProviderError;
use super::types::{ColumnBlock, Mask, RowBatch};

/// Provider stays in place for later `prop(...)` reintegration.
pub trait Provider {
    fn get_prop<'a>(
        &'a self,
        prop: &'a Property,
        batch: RowBatch<'a>,
        mask: Option<&'a Mask>,
    ) -> impl Future<Output = Result<ColumnBlock, ProviderError>> + 'a;

    fn now_epoch_ms(&self) -> i64 {
        0
    }
}
