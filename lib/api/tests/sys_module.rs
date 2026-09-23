#[cfg(all(feature = "sys", feature = "universal", feature = "singlepass"))]
mod archive_tests {
    use wasmer::{imports, Instance, Module, Store, Value};
    use wasmer_compiler_singlepass::Singlepass;
    use wasmer_engine_universal::Universal;

    fn compiled_module() -> Module {
        let store = Store::new(&Universal::new(Singlepass::default()).engine());
        let wasm = wat::parse_str(
            r#"(module $archive_control
                (memory (export "memory") 1)
                (data (i32.const 0) "\2a\00\00\00")
                (func (param i32))
                (func (export "answer") (result i32)
                    i32.const 0 i32.load))"#,
        )
        .unwrap();
        Module::new(&store, wasm).unwrap()
    }

    #[test]
    fn roundtrip_executes_with_headless_engine() {
        let module = compiled_module();
        let original = Instance::new(&module, &imports! {}).unwrap();
        let expected = original
            .exports
            .get_function("answer")
            .unwrap()
            .call(&[])
            .unwrap();
        let bytes = module.serialize().unwrap();
        let store = Store::new(&Universal::headless().engine());
        // These bytes were produced by the trusted compiler in this test.
        let restored = unsafe { Module::deserialize(&store, &bytes).unwrap() };
        assert_eq!(restored.name(), Some("archive_control"));
        assert_eq!(
            restored.exports().collect::<Vec<_>>(),
            module.exports().collect::<Vec<_>>()
        );
        let instance = Instance::new(&restored, &imports! {}).unwrap();
        let answer = instance
            .exports
            .get_function("answer")
            .unwrap()
            .call(&[])
            .unwrap();
        assert_eq!(&*answer, &[Value::I32(42)]);
        assert_eq!(answer, expected);
    }

    #[test]
    fn old_archive_version_is_rejected() {
        let mut bytes = compiled_module().serialize().unwrap();
        // Universal magic (16 bytes), metadata magic (8), then native ABI version.
        bytes[24..28].copy_from_slice(&1u32.to_ne_bytes());
        let store = Store::new(&Universal::headless().engine());
        let error = unsafe { Module::deserialize(&store, &bytes) }.unwrap_err();
        assert!(error.to_string().contains("incompatible version"));
    }

    #[test]
    fn truncated_archive_is_rejected() {
        let mut bytes = compiled_module().serialize().unwrap();
        bytes.pop();
        let store = Store::new(&Universal::headless().engine());
        let error = unsafe { Module::deserialize(&store, &bytes) }.unwrap_err();
        assert!(error.to_string().contains("truncated metadata"));
    }
}

#[cfg(feature = "sys")]
mod sys {
    use anyhow::Result;
    use wasmer::*;

    #[test]
    fn module_get_name() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module)"#;
        let module = Module::new(&store, wat)?;
        assert_eq!(module.name(), None);

        Ok(())
    }

    #[test]
    fn module_set_name() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module $name)"#;
        let mut module = Module::new(&store, wat)?;
        assert_eq!(module.name(), Some("name"));

        module.set_name("new_name");
        assert_eq!(module.name(), Some("new_name"));

        Ok(())
    }

    #[test]
    fn imports() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (import "host" "func" (func))
    (import "host" "memory" (memory 1))
    (import "host" "table" (table 1 anyfunc))
    (import "host" "global" (global i32))
)"#;
        let module = Module::new(&store, wat)?;
        assert_eq!(
            module.imports().collect::<Vec<_>>(),
            vec![
                ImportType::new(
                    "host",
                    "func",
                    ExternType::Function(FunctionType::new(vec![], vec![]))
                ),
                ImportType::new(
                    "host",
                    "memory",
                    ExternType::Memory(MemoryType::new(Pages(1), None, false))
                ),
                ImportType::new(
                    "host",
                    "table",
                    ExternType::Table(TableType::new(Type::FuncRef, 1, None))
                ),
                ImportType::new(
                    "host",
                    "global",
                    ExternType::Global(GlobalType::new(Type::I32, Mutability::Const))
                )
            ]
        );

        // Now we test the iterators
        assert_eq!(
            module.imports().functions().collect::<Vec<_>>(),
            vec![ImportType::new(
                "host",
                "func",
                FunctionType::new(vec![], vec![])
            ),]
        );
        assert_eq!(
            module.imports().memories().collect::<Vec<_>>(),
            vec![ImportType::new(
                "host",
                "memory",
                MemoryType::new(Pages(1), None, false)
            ),]
        );
        assert_eq!(
            module.imports().tables().collect::<Vec<_>>(),
            vec![ImportType::new(
                "host",
                "table",
                TableType::new(Type::FuncRef, 1, None)
            ),]
        );
        assert_eq!(
            module.imports().globals().collect::<Vec<_>>(),
            vec![ImportType::new(
                "host",
                "global",
                GlobalType::new(Type::I32, Mutability::Const)
            ),]
        );
        Ok(())
    }

    #[test]
    fn exports() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (func (export "func") nop)
    (memory (export "memory") 1)
    (table (export "table") 1 funcref)
    (global (export "global") i32 (i32.const 0))
)"#;
        let module = Module::new(&store, wat)?;
        assert_eq!(
            module.exports().collect::<Vec<_>>(),
            vec![
                ExportType::new(
                    "func",
                    ExternType::Function(FunctionType::new(vec![], vec![]))
                ),
                ExportType::new(
                    "memory",
                    ExternType::Memory(MemoryType::new(Pages(1), None, false))
                ),
                ExportType::new(
                    "table",
                    ExternType::Table(TableType::new(Type::FuncRef, 1, None))
                ),
                ExportType::new(
                    "global",
                    ExternType::Global(GlobalType::new(Type::I32, Mutability::Const))
                )
            ]
        );

        // Now we test the iterators
        assert_eq!(
            module.exports().functions().collect::<Vec<_>>(),
            vec![ExportType::new("func", FunctionType::new(vec![], vec![])),]
        );
        assert_eq!(
            module.exports().memories().collect::<Vec<_>>(),
            vec![ExportType::new(
                "memory",
                MemoryType::new(Pages(1), None, false)
            ),]
        );
        assert_eq!(
            module.exports().tables().collect::<Vec<_>>(),
            vec![ExportType::new(
                "table",
                TableType::new(Type::FuncRef, 1, None)
            ),]
        );
        assert_eq!(
            module.exports().globals().collect::<Vec<_>>(),
            vec![ExportType::new(
                "global",
                GlobalType::new(Type::I32, Mutability::Const)
            ),]
        );
        Ok(())
    }

    #[test]
    fn calling_host_functions_with_negative_values_works() -> Result<()> {
        let store = Store::default();
        let wat = r#"(module
    (import "host" "host_func1" (func (param i64)))
    (import "host" "host_func2" (func (param i32)))
    (import "host" "host_func3" (func (param i64)))
    (import "host" "host_func4" (func (param i32)))
    (import "host" "host_func5" (func (param i32)))
    (import "host" "host_func6" (func (param i32)))
    (import "host" "host_func7" (func (param i32)))
    (import "host" "host_func8" (func (param i32)))

    (func (export "call_host_func1")
          (call 0 (i64.const -1)))
    (func (export "call_host_func2")
          (call 1 (i32.const -1)))
    (func (export "call_host_func3")
          (call 2 (i64.const -1)))
    (func (export "call_host_func4")
          (call 3 (i32.const -1)))
    (func (export "call_host_func5")
          (call 4 (i32.const -1)))
    (func (export "call_host_func6")
          (call 5 (i32.const -1)))
    (func (export "call_host_func7")
          (call 6 (i32.const -1)))
    (func (export "call_host_func8")
          (call 7 (i32.const -1)))
)"#;
        let module = Module::new(&store, wat)?;
        let imports = imports! {
            "host" => {
                "host_func1" => Function::new_native(&store, |p: u64| {
                    println!("host_func1: Found number {}", p);
                    assert_eq!(p, u64::max_value());
                }),
                "host_func2" => Function::new_native(&store, |p: u32| {
                    println!("host_func2: Found number {}", p);
                    assert_eq!(p, u32::max_value());
                }),
                "host_func3" => Function::new_native(&store, |p: i64| {
                    println!("host_func3: Found number {}", p);
                    assert_eq!(p, -1);
                }),
                "host_func4" => Function::new_native(&store, |p: i32| {
                    println!("host_func4: Found number {}", p);
                    assert_eq!(p, -1);
                }),
                "host_func5" => Function::new_native(&store, |p: i16| {
                    println!("host_func5: Found number {}", p);
                    assert_eq!(p, -1);
                }),
                "host_func6" => Function::new_native(&store, |p: u16| {
                    println!("host_func6: Found number {}", p);
                    assert_eq!(p, u16::max_value());
                }),
                "host_func7" => Function::new_native(&store, |p: i8| {
                    println!("host_func7: Found number {}", p);
                    assert_eq!(p, -1);
                }),
                "host_func8" => Function::new_native(&store, |p: u8| {
                    println!("host_func8: Found number {}", p);
                    assert_eq!(p, u8::max_value());
                }),
            }
        };
        let instance = Instance::new(&module, &imports)?;

        let f1: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func1")?;
        let f2: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func2")?;
        let f3: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func3")?;
        let f4: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func4")?;
        let f5: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func5")?;
        let f6: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func6")?;
        let f7: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func7")?;
        let f8: NativeFunc<(), ()> = instance.exports.get_native_function("call_host_func8")?;

        f1.call()?;
        f2.call()?;
        f3.call()?;
        f4.call()?;
        f5.call()?;
        f6.call()?;
        f7.call()?;
        f8.call()?;

        Ok(())
    }
}
