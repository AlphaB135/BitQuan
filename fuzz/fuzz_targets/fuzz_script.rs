#![no_main]

use libfuzzer_sys::fuzz_target;
use bitquan_consensus::script::ScriptInterpreter;
use bq_crypto::CryptoRegistry;

fuzz_target!(|data: &[u8]| {
    // Fuzz script execution with arbitrary bytecode
    if !data.is_empty() && data.len() <= 10_000 {
        let script = data.to_vec();
        let message = b"test message";
        let registry = CryptoRegistry::new();
        let mut interpreter = ScriptInterpreter::new(registry);
        
        // Execute script and ensure it doesn't panic
        let _ = interpreter.execute(&script, message);
    }
    
    // Fuzz with split script/message
    if data.len() >= 2 {
        let split = (data[0] as usize) % data.len();
        let script = &data[..split];
        let message = &data[split..];
        
        if script.len() <= 10_000 {
            let registry = CryptoRegistry::new();
            let mut interpreter = ScriptInterpreter::new(registry);
            
            // Test execution doesn't panic
            let _ = interpreter.execute(script, message);
        }
    }
});
