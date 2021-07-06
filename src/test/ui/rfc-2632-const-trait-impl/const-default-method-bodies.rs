// TODO fix this test

#![feature(const_trait_impl)]
#![allow(incomplete_features)]

trait ConstDefaultFn: Sized {
    fn b(self);

    #[default_method_body_is_const]
    fn a(self) {
        self.b();
    }
}

struct NonConstImpl;
struct ConstImpl;

impl ConstDefaultFn for NonConstImpl {
    fn b(self) {}
}

impl const ConstDefaultFn for ConstImpl {
    fn b(self) {}
}

const fn test() {
    NonConstImpl.a();
    //~^ ERROR calls in constant functions are limited to constant functions, tuple structs and tuple variants
    ConstImpl.a();
}

fn main() {}
