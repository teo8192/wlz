pub mod ffi;
pub mod wlz;
pub mod wrapper;

pub use memoffset;

#[cfg(test)]
mod tests {
    use std::{mem::MaybeUninit, ptr::null_mut};

    use wlz_macros::{initialization, WlListeners};

    use crate::wrapper::wl::{Listener, Signal};

    struct Data {
        a: u32,
        b: usize,
        c: Box<u32>,
    }

    impl Data {
        fn selftest(&self) {
            assert_eq!(self.a, 0);
            assert_eq!(self.b, 1);
            assert_eq!(*self.c, 2);
        }
    }

    #[derive(WlListeners)]
    struct ListenerTest {
        num_a: u32,
        num_b: u32,
        #[listener("with_data_test", Data)]
        with_data_test: Listener,

        #[listener("without_data_test")]
        without_data_test: Listener,

        num_c: u32,
        num_d: u32,
    }

    impl ListenerTest {
        #[initialization]
        fn init(&mut self) {
            self.init_with_data_test();
            self.init_without_data_test();

            self.num_a = 0;
            self.num_b = 1;
            self.num_c = 2;
            self.num_d = 3;
        }

        fn selftest(&self) {
            assert_eq!(self.num_a, 0);
            assert_eq!(self.num_b, 1);
            assert_eq!(self.num_c, 2);
            assert_eq!(self.num_d, 3);
        }

        fn with_data_test(&mut self, data: &mut Data) {
            self.selftest();
            data.selftest();
        }

        fn without_data_test(&mut self) {
            self.selftest();
        }
    }

    #[test]
    fn trampoline_test() {
        // setup
        let lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt);
        let lt = unsafe { lt.as_mut().get_unchecked_mut() };
        let mut signal = Signal::empty();
        signal.init();
        signal.add(&mut lt.without_data_test);

        // do emit signal to do method call
        signal.emit(null_mut());
    }
}
