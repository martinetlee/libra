// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{hash, primitive_helpers, signature, vector};
use crate::value::Local;
use std::collections::{HashMap, VecDeque};
use vm::file_format::{FunctionSignature, SignatureToken, StructHandleIndex};

/// Enum representing the result of running a native function
pub enum NativeReturnStatus {
    /// Represents a successful execution.
    Success {
        /// The cost for running that function
        cost: u64,
        /// The `Vec<Local>` values will be pushed on the stack
        return_values: Vec<Local>,
    },
    /// Represents the execution of an abort instruction with the given error code
    Aborted {
        /// The cost for running that function up to the point of the abort
        cost: u64,
        /// The error code aborted on
        error_code: u64,
    },
    /// `InvalidArguments` should not occur unless there is some error in the bytecode verifier
    InvalidArguments,
}

pub struct NativeFunction {
    /// Given the vector of aguments, it executes the native function
    pub dispatch: fn(VecDeque<Local>) -> NativeReturnStatus,
    /// The signature as defined in it's declaring module.
    /// It should NOT be generally inspected outside of it's declaring module as the various
    /// struct handle indexes are not remapped into the local context
    pub expected_signature: FunctionSignature,
}

impl NativeFunction {
    /// Returns the number of arguments to the native function, derived from the expected signature
    pub fn num_args(&self) -> usize {
        self.expected_signature.arg_types.len()
    }
}

pub fn dispatch_native_function(
    module_name: &str,
    function_name: &str,
) -> Option<&'static NativeFunction> {
    NATIVE_FUNCTION_MAP.get(module_name)?.get(function_name)
}

macro_rules! add {
    ($m:ident, $module:expr, $name:expr, $dis:expr, $args:expr, $ret:expr) => {{
        let expected_signature = FunctionSignature {
            return_types: $ret,
            arg_types: $args,
            kind_constraints: vec![],
        };
        let f = NativeFunction {
            dispatch: $dis,
            expected_signature,
        };
        $m.entry($module.into())
            .or_insert_with(HashMap::new)
            .insert($name.into(), f);
    }};
}

type NativeFunctionMap = HashMap<String, HashMap<String, NativeFunction>>;

lazy_static! {
    static ref NATIVE_FUNCTION_MAP: NativeFunctionMap = {
        use SignatureToken::*;
        let mut m: NativeFunctionMap = HashMap::new();
        // Hash
        add!(m, "Hash", "keccak256",
            hash::native_keccak_256,
            vec![ByteArray],
            vec![ByteArray]
        );
        add!(m, "Hash", "ripemd160",
            hash::native_ripemd_160,
            vec![ByteArray],
            vec![ByteArray]
        );
        add!(m, "Hash", "sha2_256",
            hash::native_sha2_256,
            vec![ByteArray],
            vec![ByteArray]
        );
        add!(m, "Hash", "sha3_256",
            hash::native_sha3_256,
            vec![ByteArray],
            vec![ByteArray]
        );
        // Signature
        add!(m, "Signature", "ed25519_verify",
            signature::native_ed25519_signature_verification,
            vec![ByteArray, ByteArray, ByteArray],
            vec![Bool]
        );
        add!(m, "Signature", "ed25519_threshold_verify",
            signature::native_ed25519_threshold_signature_verification,
            vec![ByteArray, ByteArray, ByteArray, ByteArray],
            vec![U64]
        );
        // AddressUtil
        add!(m, "AddressUtil", "address_to_bytes",
            primitive_helpers::native_address_to_bytes,
            vec![Address],
            vec![ByteArray]
        );
        // U64Util
        add!(m, "U64Util", "u64_to_bytes",
            primitive_helpers::native_u64_to_bytes,
            vec![U64],
            vec![ByteArray]
        );
        // BytearrayUtil
        add!(m, "BytearrayUtil", "bytearray_concat",
            primitive_helpers::native_bytearray_concat,
            vec![ByteArray, ByteArray],
            vec![ByteArray]
        );
        // BytearrayUtil
        add!(m, "Vector", "length",
            vector::native_length,
            vec![Reference(Box::new(Struct(StructHandleIndex(0), vec![])))],
            vec![U64]
        );
        m
    };
}

#[macro_export]
macro_rules! pop_arg {
    ($arguments:ident, $t:ty) => {{
        match $arguments.pop_back().unwrap().value_as::<$t>() {
            Some(val) => val,
            None => return NativeReturnStatus::InvalidArguments,
        }
    }};
}
