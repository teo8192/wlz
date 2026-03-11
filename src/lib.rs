pub mod ffi;
pub mod wlz;
pub mod wrapper;

pub use memoffset;

#[cfg(test)]
mod tests {
    use std::mem::{self, MaybeUninit};

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
        #[listener("with_data_test_cb", Data)]
        with_data_test: Listener,

        #[listener("without_data_test_cb")]
        without_data_test: Listener,

        #[listener("destroy_cb")]
        destroy: Listener,

        num_c: u32,
        num_d: u32,
    }

    impl ListenerTest {
        #[initialization]
        fn init(&mut self) {
            self.init_with_data_test();
            self.init_without_data_test();
            self.init_destroy();

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

        fn with_data_test_cb(&mut self, data: &mut Data) {
            self.selftest();
            data.selftest();
        }

        fn without_data_test_cb(&mut self) {
            self.selftest();
        }

        fn destroy_cb(&mut self) {
            drop(unsafe { Box::from_raw(self as *mut Self) });
        }
    }

    #[test]
    fn trampoline_test() {
        // setup
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let lt = unsafe { lt.as_mut().get_unchecked_mut() };
        let mut signal = Signal::empty();
        signal.init();
        signal.add(&mut lt.without_data_test);

        // do emit signal to do method call
        signal.emit();
    }

    #[test]
    fn multiple_listener_calls() {
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let lt = unsafe { lt.as_mut().get_unchecked_mut() };
        let mut signal = Signal::empty();
        signal.init();
        signal.add(&mut lt.without_data_test);

        // do emit signal to do method call
        signal.emit();
        signal.emit();
        signal.emit();
        signal.emit();
        signal.emit();
        signal.emit();
        signal.emit();
    }

    #[test]
    fn trampoline_with_data() {
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let lt = unsafe { lt.as_mut().get_unchecked_mut() };
        let mut signal = Signal::empty();
        signal.init();
        signal.add(&mut lt.with_data_test);

        let mut data = Data {
            a: 0,
            b: 1,
            c: Box::new(2),
        };

        // do emit signal to do method call
        signal.emit_arg(&mut data);
    }

    #[test]
    fn destruction_pattern() {
        let mut pin_box = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(pin_box.as_mut());
        let lt = unsafe { lt.as_mut().get_unchecked_mut() };
        let mut signal = Signal::empty();
        signal.init();
        signal.add(&mut lt.destroy);
        mem::forget(pin_box);

        // do emit signal to do method call
        signal.emit();
    }
}
