pub mod ffi;
pub mod wlz;
pub mod wrapper;

pub use memoffset;

/// deallocate and drop an object that is originally created with a box
/// # Safety
/// Needs to ensure that the object was actually created with a box, and to not use the pinned
/// object after this
pub unsafe fn destroy_object<T>(obj: std::pin::Pin<&mut T>) {
    let ptr = unsafe { std::pin::Pin::into_inner_unchecked(obj) } as *mut T;
    drop(unsafe { Box::from_raw(ptr) });
}

#[cfg(test)]
mod tests {
    use std::{
        mem::{self, MaybeUninit},
        pin::pin,
        pin::Pin,
    };

    use pin_project::pin_project;
    use wlz_macros::{initialization, WlListeners};

    use crate::{
        destroy_object,
        wrapper::wl::{Listener, Signal},
    };

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
    #[pin_project]
    struct ListenerTest {
        num_a: u32,
        num_b: u32,
        #[listener("with_data_test_cb", Data)]
        #[pin]
        with_data_test: Listener,

        #[listener("without_data_test_cb")]
        #[pin]
        without_data_test: Listener,

        #[listener("destroy_cb")]
        #[pin]
        destroy: Listener,

        num_c: u32,
        num_d: u32,

        counter: u32,
    }

    impl ListenerTest {
        #[initialization]
        fn init(self: &mut Pin<&mut Self>) {
            let this = self.as_mut().project();
            *this.num_a = 0;
            *this.num_b = 1;
            *this.num_c = 2;
            *this.num_d = 3;

            self.as_mut().reset_counter();
        }

        fn selftest(&self) {
            assert_eq!(self.num_a, 0);
            assert_eq!(self.num_b, 1);
            assert_eq!(self.num_c, 2);
            assert_eq!(self.num_d, 3);
        }

        fn reset_counter(self: Pin<&mut Self>) {
            *self.project().counter = 0;
        }

        fn increment_counter(self: Pin<&mut Self>) {
            *self.project().counter += 1;
        }

        fn with_data_test_cb(self: Pin<&mut Self>, data: Pin<&mut Data>) {
            self.selftest();
            data.selftest();
        }

        fn without_data_test_cb(self: Pin<&mut Self>) {
            self.selftest();
            self.increment_counter();
        }

        fn destroy_cb(self: Pin<&mut Self>) {
            unsafe { destroy_object(self) };
        }
    }

    #[test]
    fn trampoline_test() {
        // setup
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let mut signal = pin!(Signal::empty());
        signal.as_mut().init();
        signal.as_mut().add(lt.as_mut().project().without_data_test);

        // do emit signal to do method call
        signal.as_mut().emit();
    }

    #[test]
    fn multiple_listener_calls() {
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let mut signal = pin!(Signal::empty());
        signal.as_mut().init();
        signal.as_mut().add(lt.as_mut().project().without_data_test);

        // do emit signal to do method call
        signal.as_mut().emit();
        signal.as_mut().emit();
        signal.as_mut().emit();
        signal.as_mut().emit();
        signal.as_mut().emit();
        signal.as_mut().emit();
        signal.as_mut().emit();

        assert_eq!(*lt.project().counter, 7);
    }

    #[test]
    fn trampoline_with_data() {
        let mut lt = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(lt.as_mut());
        let mut signal = pin!(Signal::empty());
        signal.as_mut().init();
        signal.as_mut().add(lt.as_mut().project().with_data_test);

        let mut data = Data {
            a: 0,
            b: 1,
            c: Box::new(2),
        };

        // do emit signal to do method call
        signal.as_mut().emit_arg(&mut data);
    }

    #[test]
    fn destruction_pattern() {
        let mut pin_box = Box::pin(MaybeUninit::uninit());
        let mut lt = ListenerTest::initialize(pin_box.as_mut());
        let mut signal = pin!(Signal::empty());
        signal.as_mut().init();
        signal.as_mut().add(lt.as_mut().project().destroy);
        mem::forget(pin_box);

        // do emit signal to do method call
        signal.as_mut().emit();

        // Something cool i have realised is that when the destroy callback does a drop on the
        // box, it drops all the lists as well. This dropping of the lists removes them from any
        // signals they might be added to, so the drop will not be called twice and we avoid a
        // double free.
        // In addition, any other callback will not be called on corrupted memory! :-)
    }
}
