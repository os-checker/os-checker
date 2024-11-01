// warning: associated function is never used: `push_all`
//   --> rudra-checks-this/src/vec_push_all.rs:12:8
//    |
// 12 |     fn push_all(&mut self, to_push: &[T]) {
//    |        ^^^^^^^^
//
// 2024-11-01 20:01:54.943954 |INFO | [rudra-progress] Rudra finished
// Error (SendSyncVariance:/PhantomSendForSend/NaiveSendForSend/RelaxSend): Suspicious impl of `Send` found
// -> rudra-checks-this/src/wild_send.rs:9:1: 9:40
// unsafe impl<P: Ord> Send for Atom<P> {}
// Warning (UnsafeDataflow:/ReadFlow): Potential unsafe dataflow issue in `order_unsafe::test_order_unsafe`
// -> rudra-checks-this/src/order_unsafe.rs:10:1: 15:2
// fn test_order_unsafe<I: Iterator<Item = impl Debug>>(mut iter: I) {
//     unsafe {
//         std::ptr::read(&Box::new(1234) as *const _);
//     }
//     println!("{:?}", iter.next());
// }
//
// Error (UnsafeDataflow:/WriteFlow/VecSetLen): Potential unsafe dataflow issue in `vec_push_all::MyVec::<T>::push_all`
// -> rudra-checks-this/src/vec_push_all.rs:12:5: 23:6
// fn push_all(&mut self, to_push: &[T]) {
//         self.0.reserve(to_push.len());
//         unsafe {
//             // can't overflow because we just reserved this
//             self.0.set_len(self.0.len() + to_push.len());
//
//             for (i, x) in to_push.iter().enumerate() {
//                 // Clone might panic
//                 self.0.as_mut_ptr().offset(i as isize).write(x.clone());
//             }
//         }
//     }
//
// 2024-11-01 20:01:55.043410 |INFO | [rudra-progress] cargo rudra finished

use crate::config::Resolve;

pub fn parse(stderr: &[u8], resolve: &Resolve) -> String {
    let mut output = inner_parse(stderr);
    if !output.is_empty() {
        output.insert_str(0, &resolve.display());
    }
    output
}

fn inner_parse(stderr: &[u8]) -> String {
    // rudra doesn't provide no-color option
    let stderr = String::from_utf8(strip_ansi_escapes::strip(stderr)).unwrap();
    let mut output = String::with_capacity(stderr.len() / 2);

    // skip cargo check outputs
    let pat = "\nError (";

    let Some(pos) = output.find(pat) else {
        return String::new();
    };

    output.replace_range(pos.., "");
    output
}
