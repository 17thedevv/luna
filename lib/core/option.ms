extern fn __mellis_panic();
export enum Option<T> {
    None,
    Some(T),
}

impl<T> Option<T> {
    export fn is_some(self: &Self) -> bool {
        match *self {
            Option::Some(_) -> true,
            Option::None -> false,
        }
    }

    export fn is_none(self: &Self) -> bool {
        !self.is_some()
    }

    export fn unwrap(self: Self) -> T {
        match self {
            Option::Some(val) -> val,
            Option::None -> {
                __mellis_panic()
            }
        }
    }

    export fn unwrap_or(self: Self, default: T) -> T {
        match self {
            Option::Some(val) -> val,
            Option::None -> default,
        }
    }

    export fn map<U>(self: Self, f: fn(T) -> U) -> Option<U> {
        match self {
            Option::Some(val) -> Option::Some(f(val)),
            Option::None -> Option::None,
        }
    }

    export fn and_then<U>(self: Self, f: fn(T) -> Option<U>) -> Option<U> {
        match self {
            Option::Some(val) -> f(val),
            Option::None -> Option::None,
        }
    }
}
