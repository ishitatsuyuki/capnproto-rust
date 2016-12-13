// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use futures::{Future, Stream};
use futures::sync::oneshot;

use std::cell::{RefCell};
use std::rc::{Rc};

pub struct TaskSet<T, E> {
    sender: ::futures::sync::mpsc::UnboundedSender<Box<Future<Item=(),Error=()>>>,
    _canceler: ::futures::sync::oneshot::Sender<()>, // when dropped, the tasks get canceled
    reaper: Rc<RefCell<Box<TaskReaper<T, E>>>>,
}

impl<T, E> TaskSet<T, E> where T: 'static, E: 'static {
    pub fn new(reaper: Box<TaskReaper<T, E>>, handle: &::tokio_core::reactor::Handle)
               -> TaskSet<T, E>
        where E: 'static, T: 'static, E: ::std::fmt::Debug,
    {
        let (tx, rx) = ::futures::sync::mpsc::unbounded::<Box<Future<Item=(),Error=()>>>();
        let stream = rx.map_err(|()| unreachable!())
            .buffer_unordered(1000); // XXX hack that should basically work in small cases.

        let (fulfiller, dropped) = oneshot::channel::<()>();
        let dropped = dropped.map_err(|_| ());

        let f = dropped.join(
            stream.for_each(|()| Ok(()) ).map_err(|e| { println!("error {:?}", e); ()})).map(|_| {println!("task set done"); ()});

        handle.spawn(f);

        TaskSet {
            sender: tx,
            _canceler: fulfiller,
            reaper: Rc::new(RefCell::new(reaper)),
        }
    }

    pub fn add<F>(&mut self, promise: F)
        where F: Future<Item=T, Error=E> + 'static
    {
        let reaper = self.reaper.clone();
        let task = promise.then(move |r| {
            match r {
                Ok(v) => reaper.borrow_mut().task_succeeded(v),
                Err(e) => reaper.borrow_mut().task_failed(e),
            }
            Ok(())
        });
        self.sender.send(Box::new(task)).unwrap();
    }
}


pub trait TaskReaper<T, E> where T: 'static, E: 'static
{
    fn task_succeeded(&mut self, _value: T) {}
    fn task_failed(&mut self, error: E);
}
