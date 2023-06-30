use apple_sys::ScreenCaptureKit::{
    INSArray, NSArray, NSArray_NSExtendedArray, SCDisplay, SCRunningApplication, SCWindow,
};

#[macro_export]
macro_rules! objc_closure {
    ($a:expr) => {
        &*block::ConcreteBlock::new($a).copy() as *const Block<_, _> as *mut c_void
    };
}
pub use objc_closure;

#[macro_export]
macro_rules! from_nsstring {
    ($s:expr) => {
        std::ffi::CStr::from_ptr($s.cString()).to_str().unwrap()
    };
}
pub use from_nsstring;

#[macro_export]
macro_rules! from_nsarray {
    ($T:ident, $e:expr) => {
        <Vec<$T>>::from_nsarray($e)
    };
}
pub use from_nsarray;

#[derive(Debug)]
pub struct UnsafeSendable<T>(pub T);

unsafe impl<T> Send for UnsafeSendable<T> {}

pub trait FromNSArray<T> {
    fn from_nsarray(array: NSArray) -> Vec<T>;
}

pub trait ToNSArray<T> {
    fn to_nsarray(&self) -> NSArray;
}

pub fn new_nsarray<T: 'static>() -> NSArray {
    unsafe { NSArray(<NSArray as INSArray<T>>::init(&NSArray::alloc())) }
}

macro_rules! impl_from_to_nsarray_for {
    ($T:ident) => {
        impl FromNSArray<$T> for Vec<$T> {
            fn from_nsarray(array: NSArray) -> Vec<$T> {
                let mut vec = Vec::new();
                let count = unsafe { <NSArray as INSArray<$T>>::count(&array) };
                for i in 0..count {
                    vec.push(unsafe { $T(<NSArray as INSArray<$T>>::objectAtIndex_(&array, i)) });
                }
                vec
            }
        }

        impl ToNSArray<$T> for Vec<$T> {
            fn to_nsarray(&self) -> NSArray {
                unsafe {
                    let mut array = new_nsarray::<$T>();
                    for x in self {
                        array = <NSArray as NSArray_NSExtendedArray<$T>>::arrayByAddingObject_(
                            &array, x.0,
                        );
                    }
                    array
                }
            }
        }
    };
}

impl_from_to_nsarray_for!(SCRunningApplication);
impl_from_to_nsarray_for!(SCDisplay);
impl_from_to_nsarray_for!(SCWindow);
