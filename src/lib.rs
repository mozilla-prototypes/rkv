// Copyright 2018 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

//! a simple, humane, typed Rust interface to [LMDB](http://www.lmdb.tech/doc/)
//!
//! It aims to achieve the following:
//!
//! - Avoid LMDB's sharp edges (e.g., obscure error codes for common situations).
//! - Report errors via [failure](https://docs.rs/failure/).
//! - Correctly restrict access to one handle per process via a [Manager](struct.Manager.html).
//! - Use Rust's type system to make single-typed key stores (including LMDB's own integer-keyed stores)
//!   safe and ergonomic.
//! - Encode and decode values via [bincode](https://docs.rs/bincode/)/[serde](https://docs.rs/serde/)
//!   and type tags, achieving platform-independent storage and input/output flexibility.
//!
//! It exposes these primary abstractions:
//!
//! - [Manager](struct.Manager.html): a singleton that controls access to LMDB environments
//! - [Rkv](struct.Rkv.html): an LMDB environment, which contains a set of key/value databases
//! - [Store](struct.Store.html): an LMDB database, which contains a set of key/value pairs
//!
//! Keys can be anything that implements `AsRef<[u8]>` or integers
//!  (when accessing an [IntegerStore](struct.IntegerStore.html)).
//! Values can be any of the types defined by the [Value](value/enum.Value.html) enum, including:
//!
//! - booleans (`Value::Bool`)
//! - integers (`Value::I64`, `Value::U64`)
//! - floats (`Value::F64`)
//! - strings (`Value::Str`)
//! - blobs (`Value::Blob`)
//!
//! See [Value](value/enum.Value.html) for the complete list of supported types.
//!
//! ## Basic Usage
//! ```
//! use rkv::{Manager, Rkv, SingleStore, Value, Transaction, StoreOptions};
//! use std::fs;
//! use tempfile::Builder;
//!
//! // First determine the path to the environment, which is represented
//! // on disk as a directory containing two files:
//! //
//! //   * a data file containing the key/value stores
//! //   * a lock file containing metadata about current transactions
//! //
//! // In this example, we use the `tempfile` crate to create the directory.
//! //
//! let root = Builder::new().prefix("simple-db").tempdir().unwrap();
//! fs::create_dir_all(root.path()).unwrap();
//! let path = root.path();
//!
//! // The Manager enforces that each process opens the same environment
//! // at most once by caching a handle to each environment that it opens.
//! // Retrieve the handle to an opened environment—or create one if it hasn't
//! // already been opened—by calling `Manager.get_or_create()`, passing it
//! // an `Rkv` method that opens an environment (`Rkv::new` in this case):
//! let created_arc = Manager::singleton().write().unwrap().get_or_create(path, Rkv::new).unwrap();
//! let env = created_arc.read().unwrap();
//!
//! // Call `Rkv.open_or_create_default()` to get a handle to the default
//! // (unnamed) store for the environment.
//! let mut store: SingleStore = env.open_single("mydb", StoreOptions::create()).unwrap();
//!
//! {
//!     // Use a write transaction to mutate the store by calling
//!     // `Rkv.write()` to create a `Writer`.  There can be only one
//!     // writer for a given store; opening a second one will block
//!     // until the first completes.
//!     let mut writer = env.write().unwrap();
//!
//!     // Writer takes a `Store` as the first argument.
//!     // Keys are `AsRef<[u8]>`, while values are `Value` enum instances.
//!     // Use the `Blob` variant to store arbitrary collections of bytes.
//!     store.put(&mut writer, "int", &Value::I64(1234)).unwrap();
//!     store.put(&mut writer, "uint", &Value::U64(1234_u64)).unwrap();
//!     store.put(&mut writer, "float", &Value::F64(1234.0.into())).unwrap();
//!     store.put(&mut writer, "instant", &Value::Instant(1528318073700)).unwrap();
//!     store.put(&mut writer, "boolean", &Value::Bool(true)).unwrap();
//!     store.put(&mut writer, "string", &Value::Str("héllo, yöu")).unwrap();
//!     store.put(&mut writer, "json", &Value::Json(r#"{"foo":"bar", "number": 1}"#)).unwrap();
//!     store.put(&mut writer, "blob", &Value::Blob(b"blob")).unwrap();
//!
//!     // You must commit a write transaction before the writer goes out
//!     // of scope, or the transaction will abort and the data won't persist.
//!     writer.commit().unwrap();
//! }
//!
//! {
//!     // Use a read transaction to query the store by calling `Rkv.read()`
//!     // to create a `Reader`.  There can be unlimited concurrent readers
//!     // for a store, and readers never block on a writer nor other readers.
//!     let reader = env.read().expect("reader");
//!
//!     // To retrieve data, call `Reader.get()`, passing it the target store
//!     // and the key for the value to retrieve.
//!     println!("Get int {:?}", store.get(&reader, "int").unwrap());
//!     println!("Get uint {:?}", store.get(&reader, "uint").unwrap());
//!     println!("Get float {:?}", store.get(&reader, "float").unwrap());
//!     println!("Get instant {:?}", store.get(&reader, "instant").unwrap());
//!     println!("Get boolean {:?}", store.get(&reader, "boolean").unwrap());
//!     println!("Get string {:?}", store.get(&reader, "string").unwrap());
//!     println!("Get json {:?}", store.get(&reader, "json").unwrap());
//!     println!("Get blob {:?}", store.get(&reader, "blob").unwrap());
//!
//!     // Retrieving a non-existent value returns `Ok(None)`.
//!     println!("Get non-existent value {:?}", store.get(&reader, "non-existent"));
//!
//!     // A read transaction will automatically close once the reader
//!     // goes out of scope, so isn't necessary to close it explicitly,
//!     // although you can do so by calling `Reader.abort()`.
//! }
//!
//! {
//!     // Aborting a write transaction rolls back the change(s).
//!     let mut writer = env.write().unwrap();
//!     store.put(&mut writer, "foo", &Value::Str("bar")).unwrap();
//!     writer.abort();
//!
//!     let reader = env.read().expect("reader");
//!     println!("It should be None! ({:?})", store.get(&reader, "foo").unwrap());
//! }
//!
//! {
//!     // Explicitly aborting a transaction is not required unless an early
//!     // abort is desired, since both read and write transactions will
//!     // implicitly be aborted once they go out of scope.
//!     {
//!         let mut writer = env.write().unwrap();
//!         store.put(&mut writer, "foo", &Value::Str("bar")).unwrap();
//!     }
//!     let reader = env.read().expect("reader");
//!     println!("It should be None! ({:?})", store.get(&reader, "foo").unwrap());
//! }
//!
//! {
//!     // Deleting a key/value pair also requires a write transaction.
//!     let mut writer = env.write().unwrap();
//!     store.put(&mut writer, "foo", &Value::Str("bar")).unwrap();
//!     store.put(&mut writer, "bar", &Value::Str("baz")).unwrap();
//!     store.delete(&mut writer, "foo").unwrap();
//!
//!     // A write transaction also supports reading, the version of the
//!     // store that it reads includes changes it has made regardless of
//!     // the commit state of that transaction.
//!     // In the code above, "foo" and "bar" were put into the store,
//!     // then "foo" was deleted so only "bar" will return a result.
//!     println!("It should be None! ({:?})", store.get(&writer, "foo").unwrap());
//!     println!("Get bar ({:?})", store.get(&writer, "bar").unwrap());
//!     writer.commit().unwrap();
//!     let reader = env.read().expect("reader");
//!     println!("It should be None! ({:?})", store.get(&reader, "foo").unwrap());
//!     println!("Get bar {:?}", store.get(&reader, "bar").unwrap());
//!
//!     // Committing a transaction consumes the writer, preventing you
//!     // from reusing it by failing at compile time with an error.
//!     // This line would report error[E0382]: use of moved value: `writer`.
//!     // store.put(&mut writer, "baz", &Value::Str("buz")).unwrap();
//! }
//! ```

#![allow(dead_code)]

use lmdb;

pub use lmdb::{
    DatabaseFlags,
    EnvironmentBuilder,
    EnvironmentFlags,
    Error as LmdbError,
    RoTransaction,
    RwTransaction,
    Transaction,
    WriteFlags,
};

mod env;
pub mod error;
mod manager;
pub mod store;
pub mod value;

pub use lmdb::{
    Cursor,
    Database,
    Iter as LmdbIter,
    RoCursor,
};

pub use self::store::integer::{
    IntegerStore,
    PrimitiveInt,
};
pub use self::store::integermulti::MultiIntegerStore;
pub use self::store::multi::MultiStore;
pub use self::store::single::SingleStore;
pub use self::store::Options as StoreOptions;

pub use self::env::Rkv;

pub use self::error::{
    DataError,
    StoreError,
};

pub use self::manager::Manager;

pub use self::value::{
    OwnedValue,
    Value,
};
