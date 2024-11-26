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

        if self.force && !function.name.is_empty() && !function.system_name.is_empty() {
            new_function.name = function.system_name.clone();
        }

        if !function.name.is_empty() && !function.system_name.eq(&function.name) {
            return new_function; // Already Demangled
        }

        let demangled = Self::filter(&function.system_name);

        if demangled.ne(&function.system_name) {
            new_function.name = demangled;
            return new_function;
        }

        new_function.name = function.system_name.clone();
        new_function
    }

    // Filter demangles a C++ or Rust symbol name,
    // returning the human-readable C++ or Rust name.
    // If any error occurs during demangling, the input string is returned.
    fn filter(sys_name: &str) -> String {
        //Try Demangling Rust
        if let Ok(demangled) = rustc_demangle::try_demangle(sys_name) {
            return format!("{:#}", demangled);
        }

        //Try Demangling C/C++
        if let Ok(symbol) = cpp_demangle::Symbol::new(sys_name) {
            return symbol.to_string();
        }

        sys_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_already_demangled() {
        let demangler = Demangler::new(false);
        let function = Function {
            id: String::default(),
            start_line: 0,
            name: "main".to_string(),
            system_name: "main".to_string(),
            filename: "main.c".to_string(),
            name_string_index: 0,
            system_name_string_index: 0,
            filename_string_index: 0,
        };
        assert_eq!(function, demangler.demangle(&function));
    }

    #[test]
    fn test_cpp() {
        let demangler = Demangler::new(false);
        let function = Function {
            id: String::default(),
            start_line: 0,
            name: "".to_string(),
            system_name: "_ZNSaIcEC1ERKS_".to_string(),
            filename: "".to_string(),
            name_string_index: 0,
            system_name_string_index: 0,
            filename_string_index: 0,
        };
        let demangled = demangler.demangle(&function);
        assert_eq!(
            "std::allocator<char>::allocator(std::allocator<char> const&)",
            demangled.name
        );
    }

    #[test]
    fn test_rust() {
        let demangler = Demangler::new(false);
        let function = Function {
            id: String::default(),
            start_line: 0,
            name: "".to_string(),
            system_name: "_ZN11collections5slice29_$LT$impl$u20$$u5b$T$u5d$$GT$10as_mut_ptr17hf12a6d0409938c96E".to_string(),
            filename: "".to_string(),
            name_string_index: 0,
            system_name_string_index: 0,
            filename_string_index: 0,
        };
        let demangled = demangler.demangle(&function);
        assert_eq!("collections::slice::<impl [T]>::as_mut_ptr", demangled.name);
    }
}
