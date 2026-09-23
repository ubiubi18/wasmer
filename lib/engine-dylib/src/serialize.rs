use loupe::MemoryUsage;
use rkyv::{
    rancor::Error as RkyvError, Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use wasmer_compiler::{
    CompileError, CompileModuleInfo, CompiledFunctionFrameInfo, SectionIndex, Symbol,
    SymbolRegistry,
};
use wasmer_engine::DeserializeError;
use wasmer_types::entity::{EntityRef, PrimaryMap};
use wasmer_types::{FunctionIndex, LocalFunctionIndex, OwnedDataInitializer, SignatureIndex};

fn to_compile_error(err: impl Error) -> CompileError {
    CompileError::Codegen(format!("{}", err))
}

/// Serializable struct that represents the compiled metadata.
#[derive(
    Serialize,
    Deserialize,
    Debug,
    MemoryUsage,
    RkyvSerialize,
    RkyvDeserialize,
    Archive,
    PartialEq,
    Eq,
)]
pub struct ModuleMetadata {
    pub compile_info: CompileModuleInfo,
    pub function_frame_info: Option<PrimaryMap<LocalFunctionIndex, CompiledFunctionFrameInfo>>,
    pub prefix: String,
    pub data_initializers: Box<[OwnedDataInitializer]>,
    // The function body lengths (used to find function by address)
    pub function_body_lengths: PrimaryMap<LocalFunctionIndex, u64>,
    pub cpu_features: u64,
}

pub struct ModuleMetadataSymbolRegistry<'a> {
    pub prefix: &'a String,
}

impl ModuleMetadata {
    pub fn split(&'_ mut self) -> (&'_ mut CompileModuleInfo, ModuleMetadataSymbolRegistry<'_>) {
        let compile_info = &mut self.compile_info;
        let symbol_registry = ModuleMetadataSymbolRegistry {
            prefix: &self.prefix,
        };
        (compile_info, symbol_registry)
    }

    pub fn get_symbol_registry(&'_ self) -> ModuleMetadataSymbolRegistry<'_> {
        ModuleMetadataSymbolRegistry {
            prefix: &self.prefix,
        }
    }

    pub fn serialize(&mut self) -> Result<Vec<u8>, CompileError> {
        rkyv::to_bytes::<RkyvError>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(to_compile_error)
    }

    pub unsafe fn deserialize(metadata_slice: &[u8]) -> Result<Self, DeserializeError> {
        rkyv::from_bytes::<Self, RkyvError>(metadata_slice)
            .map_err(|e| DeserializeError::CorruptedBinary(e.to_string()))
    }

    pub fn deserialize_from_archive(
        archived: &ArchivedModuleMetadata,
    ) -> Result<Self, DeserializeError> {
        rkyv::deserialize::<Self, RkyvError>(archived)
            .map_err(|e| DeserializeError::CorruptedBinary(e.to_string()))
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use std::sync::Arc;
    use wasmer_types::{Features, ModuleInfo};

    #[test]
    fn metadata_roundtrip_and_truncation() {
        let mut metadata = ModuleMetadata {
            compile_info: CompileModuleInfo {
                module: Arc::new(ModuleInfo::default()),
                features: Features::default(),
                memory_styles: PrimaryMap::new(),
                table_styles: PrimaryMap::new(),
            },
            function_frame_info: None,
            prefix: "archive-control".into(),
            data_initializers: Box::new([]),
            function_body_lengths: PrimaryMap::new(),
            cpu_features: 0,
        };
        let bytes = ModuleMetadata::serialize(&mut metadata).unwrap();
        assert_eq!(
            unsafe { ModuleMetadata::deserialize(&bytes).unwrap() },
            metadata
        );
        assert!(unsafe { ModuleMetadata::deserialize(&bytes[..1]) }.is_err());
    }
}

impl<'a> SymbolRegistry for ModuleMetadataSymbolRegistry<'a> {
    fn symbol_to_name(&self, symbol: Symbol) -> String {
        match symbol {
            Symbol::LocalFunction(index) => {
                format!("wasmer_function_{}_{}", self.prefix, index.index())
            }
            Symbol::Section(index) => format!("wasmer_section_{}_{}", self.prefix, index.index()),
            Symbol::FunctionCallTrampoline(index) => {
                format!(
                    "wasmer_trampoline_function_call_{}_{}",
                    self.prefix,
                    index.index()
                )
            }
            Symbol::DynamicFunctionTrampoline(index) => {
                format!(
                    "wasmer_trampoline_dynamic_function_{}_{}",
                    self.prefix,
                    index.index()
                )
            }
        }
    }

    fn name_to_symbol(&self, name: &str) -> Option<Symbol> {
        if let Some(index) = name.strip_prefix(&format!("wasmer_function_{}_", self.prefix)) {
            index
                .parse::<u32>()
                .ok()
                .map(|index| Symbol::LocalFunction(LocalFunctionIndex::from_u32(index)))
        } else if let Some(index) = name.strip_prefix(&format!("wasmer_section_{}_", self.prefix)) {
            index
                .parse::<u32>()
                .ok()
                .map(|index| Symbol::Section(SectionIndex::from_u32(index)))
        } else if let Some(index) =
            name.strip_prefix(&format!("wasmer_trampoline_function_call_{}_", self.prefix))
        {
            index
                .parse::<u32>()
                .ok()
                .map(|index| Symbol::FunctionCallTrampoline(SignatureIndex::from_u32(index)))
        } else if let Some(index) = name.strip_prefix(&format!(
            "wasmer_trampoline_dynamic_function_{}_",
            self.prefix
        )) {
            index
                .parse::<u32>()
                .ok()
                .map(|index| Symbol::DynamicFunctionTrampoline(FunctionIndex::from_u32(index)))
        } else {
            None
        }
    }
}
