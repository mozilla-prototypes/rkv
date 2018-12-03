// Any copyright is dedicated to the Public Domain.
// http://creativecommons.org/publicdomain/zero/1.0/

//! A simple rkv demo that showcases the basic usage (put/get/delete) of rkv.
//!
//! You can test this out by running:
//!
//!     cargo run --example simple-store

extern crate rkv;
extern crate tempfile;

use rkv::{
    Manager,
    MultiStore,
    MultiWriter,
    Rkv,
    Value,
};
use tempfile::Builder;

use std::fs;

fn test_getput<'env, 's>(
    store: MultiStore,
    writer: MultiWriter<'env, &'s str>,
    ids: &'s mut Vec<String>,
) -> MultiWriter<'env, &'s str> {
    let keys = vec!["str1", "str2", "str3"];
    // we convert the writer into a cursor so that we can safely read
    let curs = writer.into_cursor();
    for k in keys.iter() {
        // this is a multi-valued database, so get returns an iterator
        let iter = curs.get(store, k).unwrap();
        for (_key, val) in iter {
            if let Value::Str(s) = val.unwrap().unwrap() {
                ids.push(s.to_owned());
            } else {
                panic!("didn't get a string back!");
            }
        }
    }
    let mut writer = curs.into_writer();
    for i in 0..ids.len() {
        let _r = writer.put(store, &ids[i], &Value::Blob(b"weeeeeee")).unwrap();
    }
    writer
}

fn test_delete<'env, 's>(
    store: MultiStore,
    mut writer: MultiWriter<'env, &'s str>,
) -> MultiWriter<'env, &'s str> {
    let keys = vec!["str1", "str2", "str3"];
    let vals = vec!["string uno", "string quatro", "string siete"];
    // we convert the writer into a cursor so that we can safely read
    for i in 0..keys.len() {
        writer.delete(store, &keys[i], &Value::Str(vals[i])).unwrap();
    }
    writer
}

fn main() {
    let root = Builder::new().prefix("simple-db").tempdir().unwrap();
    fs::create_dir_all(root.path()).unwrap();
    let p = root.path();

    // The manager enforces that each process opens the same lmdb environment at most once
    let created_arc = Manager::singleton().write().unwrap().get_or_create(p, Rkv::new).unwrap();
    let k = created_arc.read().unwrap();

    // Creates a store called "store"
    let store = k.open_or_create("store").unwrap();

    let multistore = k.open_or_create_multi("multistore").unwrap();

    println!("Inserting data...");
    {
        // Use a writer to mutate the store
        let mut writer = k.write().unwrap();
        writer.put(store, "int", &Value::I64(1234)).unwrap();
        writer.put(store, "uint", &Value::U64(1234_u64)).unwrap();
        writer.put(store, "float", &Value::F64(1234.0.into())).unwrap();
        writer.put(store, "instant", &Value::Instant(1528318073700)).unwrap();
        writer.put(store, "boolean", &Value::Bool(true)).unwrap();
        writer.put(store, "string", &Value::Str("héllo, yöu")).unwrap();
        writer.put(store, "json", &Value::Json(r#"{"foo":"bar", "number": 1}"#)).unwrap();
        writer.put(store, "blob", &Value::Blob(b"blob")).unwrap();
        writer.commit().unwrap();
    }

    println!("Testing getput");
    {
        let mut ids = Vec::new();
        let mut writer = k.write_multi().unwrap();
        writer.put(multistore, "str1", &Value::Str("string uno")).unwrap();
        writer.put(multistore, "str1", &Value::Str("string dos")).unwrap();
        writer.put(multistore, "str1", &Value::Str("string tres")).unwrap();
        writer.put(multistore, "str2", &Value::Str("string quatro")).unwrap();
        writer.put(multistore, "str2", &Value::Str("string cinco")).unwrap();
        writer.put(multistore, "str2", &Value::Str("string seis")).unwrap();
        writer.put(multistore, "str3", &Value::Str("string siete")).unwrap();
        writer.put(multistore, "str3", &Value::Str("string ocho")).unwrap();
        writer.put(multistore, "str3", &Value::Str("string nueve")).unwrap();
        let writer = test_getput(multistore, writer, &mut ids);
        writer.commit().unwrap();
        let writer = k.write_multi().unwrap();
        let writer = test_delete(multistore, writer);
        writer.commit().unwrap();
    }
    println!("Looking up keys...");
    {
        // Use a reader to query the store
        let reader = k.read().unwrap();
        println!("Get int {:?}", reader.get(store, "int").unwrap());
        println!("Get uint {:?}", reader.get(store, "uint").unwrap());
        println!("Get float {:?}", reader.get(store, "float").unwrap());
        println!("Get instant {:?}", reader.get(store, "instant").unwrap());
        println!("Get boolean {:?}", reader.get(store, "boolean").unwrap());
        println!("Get string {:?}", reader.get(store, "string").unwrap());
        println!("Get json {:?}", reader.get(store, "json").unwrap());
        println!("Get blob {:?}", reader.get(store, "blob").unwrap());
        println!("Get non-existent {:?}", reader.get(store, "non-existent").unwrap());
    }

    println!("Looking up keys via Writer.get()...");
    {
        let mut writer = k.write().unwrap();
        writer.put(store, "foo", &Value::Str("bar")).unwrap();
        writer.put(store, "bar", &Value::Str("baz")).unwrap();
        writer.delete(store, "foo").unwrap();
        println!("It should be None! ({:?})", writer.get(store, "foo").unwrap());
        println!("Get bar ({:?})", writer.get(store, "bar").unwrap());
        writer.commit().unwrap();
        let reader = k.read().expect("reader");
        println!("It should be None! ({:?})", reader.get(store, "foo").unwrap());
        println!("Get bar {:?}", reader.get(store, "bar").unwrap());
    }

    println!("Aborting transaction...");
    {
        // Aborting a write transaction rollbacks the change(s)
        let mut writer = k.write().unwrap();
        writer.put(store, "foo", &Value::Str("bar")).unwrap();
        writer.abort();

        let reader = k.read().expect("reader");
        println!("It should be None! ({:?})", reader.get(store, "foo").unwrap());
        // Explicitly aborting a transaction is not required unless an early
        // abort is desired, since both read and write transactions will
        // implicitly be aborted once they go out of scope.
    }

    println!("Deleting keys...");
    {
        // Deleting a key/value also requires a write transaction
        let mut writer = k.write().unwrap();
        writer.put(store, "foo", &Value::Str("bar")).unwrap();
        writer.delete(store, "foo").unwrap();
        println!("It should be None! ({:?})", writer.get(store, "foo").unwrap());
        writer.commit().unwrap();

        // Committing a transaction consumes the writer, preventing you
        // from reusing it by failing and reporting a compile-time error.
        // This line would report error[E0382]: use of moved value: `writer`.
        // writer.put(store, "baz", &Value::Str("buz")).unwrap();
    }

    println!("Write and read on multiple stores...");
    {
        let another_store = k.open_or_create("another_store").unwrap();
        let mut writer = k.write().unwrap();
        writer.put(store, "foo", &Value::Str("bar")).unwrap();
        writer.put(another_store, "foo", &Value::Str("baz")).unwrap();
        writer.commit().unwrap();

        let reader = k.read().unwrap();
        println!("Get from store value: {:?}", reader.get(store, "foo").unwrap());
        println!("Get from another store value: {:?}", reader.get(another_store, "foo").unwrap());
    }
}
