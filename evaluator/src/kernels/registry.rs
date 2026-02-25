use std::sync::LazyLock;

use crate::core::errors::EvalError;
use crate::core::types::{EvalBlock, Mask};
use crate::ir::nodes::{BINARY_EXEC_KEY_COUNT, BinaryExecKey};

use super::arithmetic::{exec_add_any, exec_add_f64, exec_div_f64, exec_mul_f64, exec_sub_f64};
use super::prepared::{PreparedArgs, prepare_any_args, prepare_f64_args};

pub(crate) type PrepareBinaryFn =
    for<'a> fn(&'a EvalBlock, &'a EvalBlock, &Mask) -> Result<PreparedArgs<'a>, EvalBlock>;
pub(crate) type ExecBinaryFn = for<'a> fn(PreparedArgs<'a>, &Mask, Vec<(usize, EvalError)>) -> EvalBlock;

#[derive(Clone, Copy)]
pub(crate) struct KernelEntry {
    pub prepare: PrepareBinaryFn,
    pub exec: ExecBinaryFn,
}

pub(crate) struct BinaryKernelRegistry {
    entries: [KernelEntry; BINARY_EXEC_KEY_COUNT],
}

impl BinaryKernelRegistry {
    fn new() -> Self {
        Self {
            entries: [
                KernelEntry {
                    prepare: prepare_f64_args,
                    exec: exec_add_f64,
                },
                KernelEntry {
                    prepare: prepare_any_args,
                    exec: exec_add_any,
                },
                KernelEntry {
                    prepare: prepare_f64_args,
                    exec: exec_sub_f64,
                },
                KernelEntry {
                    prepare: prepare_f64_args,
                    exec: exec_mul_f64,
                },
                KernelEntry {
                    prepare: prepare_f64_args,
                    exec: exec_div_f64,
                },
            ],
        }
    }

    pub(crate) fn get(&self, key: BinaryExecKey) -> KernelEntry {
        self.entries[key as usize]
    }
}

pub(crate) static BINARY_KERNEL_REGISTRY: LazyLock<BinaryKernelRegistry> =
    LazyLock::new(BinaryKernelRegistry::new);
