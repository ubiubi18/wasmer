use loupe::MemoryUsage;
use rkyv::{rancor::Error, Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use wasmer_artifact::{DeserializeError, SerializeError};
use wasmer_compiler::{
    CompileModuleInfo, CompiledFunctionFrameInfo, CustomSection, Dwarf, FunctionBody, Relocation,
    SectionIndex,
};
use wasmer_types::entity::PrimaryMap;
use wasmer_types::{FunctionIndex, LocalFunctionIndex, OwnedDataInitializer, SignatureIndex};

/// The compilation related data for a serialized modules
#[derive(MemoryUsage, Archive, RkyvDeserialize, RkyvSerialize)]
pub struct SerializableCompilation {
    pub function_bodies: PrimaryMap<LocalFunctionIndex, FunctionBody>,
    pub function_relocations: PrimaryMap<LocalFunctionIndex, Vec<Relocation>>,
    pub function_frame_info: PrimaryMap<LocalFunctionIndex, CompiledFunctionFrameInfo>,
    pub function_call_trampolines: PrimaryMap<SignatureIndex, FunctionBody>,
    pub dynamic_function_trampolines: PrimaryMap<FunctionIndex, FunctionBody>,
    pub custom_sections: PrimaryMap<SectionIndex, CustomSection>,
    pub custom_section_relocations: PrimaryMap<SectionIndex, Vec<Relocation>>,
    // The section indices corresponding to the Dwarf debug info
    pub debug: Option<Dwarf>,
    // Custom section containing libcall trampolines.
    pub libcall_trampolines: SectionIndex,
    // Length of each libcall trampoline.
    pub libcall_trampoline_len: u32,
}

/// Serializable struct that is able to serialize from and to
/// a `UniversalArtifactInfo`.
#[derive(MemoryUsage, Archive, RkyvDeserialize, RkyvSerialize)]
pub struct SerializableModule {
    /// The main serializable compilation object
    pub compilation: SerializableCompilation,
    /// Compilation informations
    pub compile_info: CompileModuleInfo,
    /// Datas initializers
    pub data_initializers: Box<[OwnedDataInitializer]>,
    /// CPU Feature flags for this compilation
    pub cpu_features: u64,
}

fn to_serialize_error(err: impl std::error::Error) -> SerializeError {
    SerializeError::Generic(format!("{}", err))
}

impl SerializableModule {
    /// Serialize a Module into bytes
    /// Uses the rkyv 0.8 format, with the archived root at the end.
    pub fn serialize(&self) -> Result<Vec<u8>, SerializeError> {
        rkyv::to_bytes::<Error>(self)
            .map(|bytes| bytes.to_vec())
            .map_err(to_serialize_error)
    }

    /// Deserialize a Module from a slice.
    /// The slice must contain a rkyv 0.8 archive.
    ///
    /// # Safety
    ///
    /// Archive structure is validated, including shared pointer metadata.
    /// The compiled code and its semantic invariants must still be trusted.
    pub unsafe fn deserialize(metadata_slice: &[u8]) -> Result<Self, DeserializeError> {
        rkyv::from_bytes::<Self, Error>(metadata_slice)
            .map_err(|e| DeserializeError::CorruptedBinary(e.to_string()))
    }

    /// Deserialize a compilation module from an archive
    pub fn deserialize_from_archive(
        archived: &ArchivedSerializableModule,
    ) -> Result<Self, DeserializeError> {
        rkyv::deserialize::<Self, Error>(archived)
            .map_err(|e| DeserializeError::CorruptedBinary(e.to_string()))
    }
}
