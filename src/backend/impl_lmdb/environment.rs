// Copyright 2018-2019 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use std::{
    fs,
    path::Path,
};

use super::{
    DatabaseFlagsImpl,
    DatabaseImpl,
    EnvironmentFlagsImpl,
    ErrorImpl,
    InfoImpl,
    RoTransactionImpl,
    RwTransactionImpl,
    StatImpl,
};
use crate::backend::traits::{
    BackendEnvironment,
    BackendEnvironmentBuilder,
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct EnvironmentBuilderImpl {
    builder: lmdb::EnvironmentBuilder,
    make_dir: bool,
}

impl<'b> BackendEnvironmentBuilder<'b> for EnvironmentBuilderImpl {
    type Environment = EnvironmentImpl;
    type Error = ErrorImpl;
    type Flags = EnvironmentFlagsImpl;

    fn new() -> EnvironmentBuilderImpl {
        EnvironmentBuilderImpl {
            builder: lmdb::Environment::new(),
            make_dir: false,
        }
    }

    fn set_flags<T>(&mut self, flags: T) -> &mut Self
    where
        T: Into<Self::Flags>,
    {
        self.builder.set_flags(flags.into().0);
        self
    }

    fn set_max_readers(&mut self, max_readers: u32) -> &mut Self {
        self.builder.set_max_readers(max_readers);
        self
    }

    fn set_max_dbs(&mut self, max_dbs: u32) -> &mut Self {
        self.builder.set_max_dbs(max_dbs);
        self
    }

    fn set_map_size(&mut self, size: usize) -> &mut Self {
        self.builder.set_map_size(size);
        self
    }

    fn set_make_dir_if_needed(&mut self, make_dir: bool) -> &mut Self {
        self.make_dir = make_dir;
        self
    }

    fn open(&self, path: &Path) -> Result<Self::Environment, Self::Error> {
        if !path.is_dir() {
            if !self.make_dir {
                return Err(ErrorImpl::DirectoryDoesNotExistError(path.into()));
            }
            fs::create_dir_all(path).map_err(ErrorImpl::IoError)?;
        }
        self.builder.open(path).map(EnvironmentImpl).map_err(ErrorImpl::LmdbError)
    }
}

#[derive(Debug)]
pub struct EnvironmentImpl(lmdb::Environment);

impl<'e> BackendEnvironment<'e> for EnvironmentImpl {
    type Database = DatabaseImpl;
    type Error = ErrorImpl;
    type Flags = DatabaseFlagsImpl;
    type Info = InfoImpl;
    type RoTransaction = RoTransactionImpl<'e>;
    type RwTransaction = RwTransactionImpl<'e>;
    type Stat = StatImpl;

    fn open_db(&self, name: Option<&str>) -> Result<Self::Database, Self::Error> {
        self.0.open_db(name).map(DatabaseImpl).map_err(ErrorImpl::LmdbError)
    }

    fn create_db(&self, name: Option<&str>, flags: Self::Flags) -> Result<Self::Database, Self::Error> {
        self.0.create_db(name, flags.0).map(DatabaseImpl).map_err(ErrorImpl::LmdbError)
    }

    fn begin_ro_txn(&'e self) -> Result<Self::RoTransaction, Self::Error> {
        self.0.begin_ro_txn().map(RoTransactionImpl).map_err(ErrorImpl::LmdbError)
    }

    fn begin_rw_txn(&'e self) -> Result<Self::RwTransaction, Self::Error> {
        self.0.begin_rw_txn().map(RwTransactionImpl).map_err(ErrorImpl::LmdbError)
    }

    fn sync(&self, force: bool) -> Result<(), Self::Error> {
        self.0.sync(force).map_err(ErrorImpl::LmdbError)
    }

    fn stat(&self) -> Result<Self::Stat, Self::Error> {
        self.0.stat().map(StatImpl).map_err(ErrorImpl::LmdbError)
    }

    fn info(&self) -> Result<Self::Info, Self::Error> {
        self.0.info().map(InfoImpl).map_err(ErrorImpl::LmdbError)
    }

    fn freelist(&self) -> Result<usize, Self::Error> {
        self.0.freelist().map_err(ErrorImpl::LmdbError)
    }

    fn set_map_size(&self, size: usize) -> Result<(), Self::Error> {
        self.0.set_map_size(size).map_err(ErrorImpl::LmdbError)
    }
}
