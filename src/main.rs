use cranelift_codegen::settings::{self, Configurable};
use cranelift_native;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use wasmtime_jit::RuntimeValue;
use wasmtime_jit::{ActionOutcome, Context};

fn main() {
    println!("Loading WASM binary.");
    let binary = File::open("./test.wasm").unwrap();

    // In order to run this binary, we need to prepare a few inputs.
    // First, we need a Context. We can build one with a ContextBuilder.
    let context_builder = ContextBuilder {
        opt_level: None,
        enable_verifier: false,
        set_debug_info: false,
    };
    let mut context = context_builder.try_build().unwrap();

    // We also need a WASM function from the binary to execute.
    let function = String::from("add");
    let function_execute = Some(&function);

    // Now, we actually execute the code via handle_module.
    println!("Running module and invoking function: {}", function);
    let result = handle_module(&mut context, binary, function_execute).unwrap();

    // Finally, let's see the result of that code.
    println!("Got result {:#?}", result);
    println!("Done.");
}

// Everything from here below was taken from @steveeJ's initial API draft here: https://github.com/steveeJ-forks/wasmtime/blob/pr/wasmtime-api/src/libwasmtime.rs
pub struct ContextBuilder<'a> {
    pub opt_level: Option<&'a str>,
    pub enable_verifier: bool,
    pub set_debug_info: bool,
}

impl<'a> ContextBuilder<'a> {
    pub fn try_build(&self) -> Result<Context, String> {
        let mut flag_builder = settings::builder();

        // Enable verifier passes in debug mode.
        if self.enable_verifier {
            flag_builder
                .enable("enable_verifier")
                .map_err(|e| e.to_string())?;
        }

        if let Some(opt_level) = self.opt_level {
            flag_builder
                .set("opt_level", opt_level)
                .map_err(|e| e.to_string())?;
        }

        let isa_builder = cranelift_native::builder()
            .map_err(|e| format!("host machine is not a supported target: {}", e))?;

        let isa = isa_builder.finish(settings::Flags::new(flag_builder));

        // set_debug_info removed because it was causing context to be empty.
        let context = Context::with_isa(isa); //set_debug_info(self.set_debug_info);

        Ok(context)
    }
}

pub fn read_wasm<T>(mut module: T) -> Result<Vec<u8>, String>
where
    T: Read,
{
    let data = {
        let mut buf: Vec<u8> = Vec::new();
        module
            .read_to_end(&mut buf)
            .map_err(|err| err.to_string())?;
        buf
    };

    // to a wasm binary with wat2wasm.
    if data.starts_with(&[b'\0', b'a', b's', b'm']) {
        Ok(data)
    } else {
        wabt::wat2wasm(data).map_err(|err| String::from(err.description()))
    }
}

pub fn handle_module<T>(
    context: &mut Context,
    module: T,
    flag_invoke: Option<&String>,
) -> Result<Option<Vec<RuntimeValue>>, String>
where
    T: std::io::Read,
{
    // Read the wasm module binary.
    let data = read_wasm(module)?;

    // Compile and instantiating a wasm module.
    let mut instance = context
        .instantiate_module(None, &data)
        .map_err(|e| e.to_string())?;

    // If a function to invoke was given, invoke it.
    if let Some(ref f) = flag_invoke {
        match context
            .invoke(&mut instance, f, &[])
            .map_err(|e| e.to_string())?
        {
            ActionOutcome::Returned { values } => Ok(Some(values)),
            ActionOutcome::Trapped { message } => {
                Err(format!("Trap from within function {}: {}", f, message))
            }
        }
    } else {
        Ok(None)
    }
}
