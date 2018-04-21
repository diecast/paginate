extern crate typemap;
extern crate diecast;

use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use std::ops::Range;

use diecast::{Bind, Item, Handle};

// TODO: should this just contain the items itself instead of the range?
#[derive(Clone)]
pub struct Page {
    pub first: (usize, Arc<PathBuf>),
    pub next: Option<(usize, Arc<PathBuf>)>,
    pub curr: (usize, Arc<PathBuf>),
    pub prev: Option<(usize, Arc<PathBuf>)>,
    pub last: (usize, Arc<PathBuf>),

    pub range: Range<usize>,

    pub page_count: usize,
    pub post_count: usize,
    pub posts_per_page: usize,
}

impl typemap::Key for Page {
    type Value = Page;
}

pub struct Paginate<R>
where R: Fn(usize) -> PathBuf, R: Sync + Send + 'static {
    target: String,
    factor: usize,
    router: R
}

impl<R> Handle<Bind> for Paginate<R>
where R: Fn(usize) -> PathBuf, R: Sync + Send + 'static {
    fn handle(&self, bind: &mut Bind) -> diecast::Result<()> {
        pages(bind, &bind.dependencies[&self.target], self.factor, &self.router);
        Ok(())
    }
}

// TODO: this should actually use a Dependency -> name trait
// we probably have to re-introduce it
#[inline]
pub fn paginate<S: Into<String>, R>(target: S, factor: usize, router: R) -> Paginate<R>
where R: Fn(usize) -> PathBuf, R: Sync + Send + 'static {
    Paginate {
        target: target.into(),
        factor: factor,
        router: router,
    }
}
// Rule::named("note index")
// .handler(chain![
//     paginate(
//         "notes", 10,
//         |n: usize| PathBuf::from(format!("notes/{}/index.html", n))),
// ])

// FIXME
// the problem with this using indices is that if the bind is sorted
// or the order is otherwise changed, the indices will no longer match!
pub fn pages<R>(destination: &mut Bind, bind: &Bind, factor: usize, router: &R)
where R: Fn(usize) -> PathBuf, R: Sync + Send + 'static {
    let post_count = bind.items().len();

    let page_count = {
        let (div, rem) = (post_count / factor, post_count % factor);

        if rem == 0 {
            div
        } else {
            div + 1
        }
    };

    if page_count == 0 {
        return;
    }

    let last_num = page_count - 1;

    let mut cache: HashMap<usize, Arc<PathBuf>> = HashMap::new();

    let mut router = |num: usize| -> Arc<PathBuf> {
        cache.entry(num)
            .or_insert_with(|| Arc::new(router(num)))
            .clone()
    };

    let first = (1, router(1));
    let last = (last_num, router(last_num));

    // grow the number of pages as needed
    for current in 0 .. page_count {
        let prev =
            if current == 0 { None }
            else { let num = current - 1; Some((num, router(num))) };
        let next =
            if current == last_num { None }
            else { let num = current + 1; Some((num, router(num))) };

        let start = current * factor;
        let end = ::std::cmp::min(post_count, (current + 1) * factor);

        let target = router(current);

        let first = first.clone();
        let last = last.clone();
        let curr = (current, target.clone());

        let page_struct =
            Page {
                first: first,

                prev: prev,
                curr: curr,
                next: next,

                last: last,

                page_count: page_count,
                post_count: post_count,
                posts_per_page: factor,

                range: start .. end,
            };

        let mut page = Item::writing((*target).clone());
        page.extensions.insert::<Page>(page_struct);
        destination.attach(page);
    }
}

