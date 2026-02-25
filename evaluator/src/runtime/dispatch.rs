use crate::core::types::{EvalBlock, Mask};
use crate::ir::nodes::BinaryExecKey;
use crate::kernels::registry::BINARY_KERNEL_REGISTRY;

pub(crate) fn dispatch_binary(
    key: BinaryExecKey,
    left: EvalBlock,
    right: EvalBlock,
    mask: &Mask,
) -> EvalBlock {
    let mut merged_errors = left.errors.clone();
    merged_errors.extend(right.errors.iter().cloned());

    let entry = BINARY_KERNEL_REGISTRY.get(key);
    let prepared = match (entry.prepare)(&left, &right, mask) {
        Ok(prepared) => prepared,
        Err(mut err_block) => {
            err_block.errors.extend(merged_errors);
            return err_block;
        }
    };

    (entry.exec)(prepared, mask, merged_errors)
}
