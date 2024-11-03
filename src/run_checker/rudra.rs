use crate::config::Resolve;
use regex::Regex;
use std::sync::LazyLock;

pub fn parse(stderr: &[u8], resolve: &Resolve) -> String {
    let mut output = inner_parse(stderr);
    if !output.is_empty() {
        output.insert_str(0, &resolve.display());
    }
    output
}

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\nError \()|(\nWarning \()").unwrap());

fn inner_parse(stderr: &[u8]) -> String {
    // rudra doesn't provide no-color option
    let mut stderr = String::from_utf8(strip_ansi_escapes::strip(stderr)).unwrap();

    // skip cargo check outputs
    let Some(pos) = RE.find(&stderr).map(|m| m.start() + 1) else {
        return String::new();
    };

    stderr.replace_range(..pos, "");
    stderr
}

#[cfg(test)]
mod tests {
    use super::*;

    const OUTPUT: &str = r#"
warning: associated function is never used: `push_all`
  --> rudra-checks-this/src/vec_push_all.rs:12:8
   |
12 |     fn push_all(&mut self, to_push: &[T]) {
   |        ^^^^^^^^

Error (SendSyncVariance:/PhantomSendForSend/NaiveSendForSend/RelaxSend): Suspicious impl of `Send` found
-> rudra-checks-this/src/wild_send.rs:9:1: 9:40
unsafe impl<P: Ord> Send for Atom<P> {}
Warning (UnsafeDataflow:/ReadFlow): Potential unsafe dataflow issue in `order_unsafe::test_order_unsafe`
-> rudra-checks-this/src/order_unsafe.rs:10:1: 15:2
fn test_order_unsafe<I: Iterator<Item = impl Debug>>(mut iter: I) {
    unsafe {
        std::ptr::read(&Box::new(1234) as *const _);
    }
    println!("{:?}", iter.next());
}

Error (UnsafeDataflow:/WriteFlow/VecSetLen): Potential unsafe dataflow issue in `vec_push_all::MyVec::<T>::push_all`
-> rudra-checks-this/src/vec_push_all.rs:12:5: 23:6
fn push_all(&mut self, to_push: &[T]) {
        self.0.reserve(to_push.len());
        unsafe {
            // can't overflow because we just reserved this
            self.0.set_len(self.0.len() + to_push.len());

            for (i, x) in to_push.iter().enumerate() {
                // Clone might panic
                self.0.as_mut_ptr().offset(i as isize).write(x.clone());
            }
        }
    }

"#;

    const OUTPUT2: &str = r#"
Warning (UnsafeDataflow:/ReadFlow):

Error (SendSyncVariance:/PhantomSendForSend/NaiveSendForSend/RelaxSend):
"#;

    #[test]
    fn rudra() {
        println!(
            "********* OUTPUT *********\n{}",
            inner_parse(OUTPUT.as_bytes())
        );
        println!(
            "\n\n********* OUTPUT2 *********\n{}",
            inner_parse(OUTPUT2.as_bytes())
        );
    }
}
