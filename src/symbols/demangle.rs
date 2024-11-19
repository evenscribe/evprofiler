use crate::metapb;

use self::metapb::Function;
/// Demangler demangles GCC/LLVM C++ and Rust symbol names.
///
/// Demangling is the inverse process of mangling (encoding of each unique
/// function and parameter list combination into a unique name for the linker).
/// With mangling the linker can tell the difference between overloaded functions
/// (they have the same name in the source code but different parameter lists).
pub struct Demangler {
    force: bool,
}

impl Demangler {
    /// Creates a new Demangler with a given demangler mode.
    ///
    /// If force is set, overwrite any names that appear already demangled.
    pub fn new(force: bool) -> Self {
        Self { force }
    }

    pub fn demangle(&self, function: &Function) -> Function {
        let mut new_function = function.clone();

        if self.force && function.name.is_empty() && function.system_name.is_empty() {
            new_function.name = function.system_name.clone();
        }

        if function.name.is_empty() && function.system_name.eq(&function.name) {
            return new_function; // Already Demangled
        }

        let demangled = Self::filter(&function.system_name);

        if demangled.ne(&function.system_name) {
            new_function.name = demangled;
            return new_function;
        }

        new_function
    }

    // Filter demangles a C++ or Rust symbol name,
    // returning the human-readable C++ or Rust name.
    // If any error occurs during demangling, the input string is returned.
    fn filter(sys_name: &str) -> String {
        //Try Demangling Rust
        let _ = match rustc_demangle::try_demangle(sys_name) {
            Ok(demangled) => return demangled.to_string(),
            Err(_) => (),
        };

        //Try Demangling C/C++
        let _ = match cpp_demangle::Symbol::new(sys_name) {
            Ok(symbol) => return symbol.to_string(),
            Err(_) => (),
        };

        sys_name.to_string()
    }
}
