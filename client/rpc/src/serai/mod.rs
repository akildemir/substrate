// This file is part of a fork of Substrate which has had various changes.

// Copyright (C) 2022-2023 Luke Parker
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Substrate Serai API.

use std::{marker::PhantomData, sync::Arc};

use sc_rpc_api::serai::*;

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_validator_sets::NetworkAddress;

use jsonrpsee::core::{Error, RpcResult};

/// Serai API
pub struct Serai<Block: BlockT, Client> {
	/// Substrate client
	client: Arc<Client>,
	_phantom: PhantomData<Block>,
}

impl<Block: BlockT, Client> Serai<Block, Client> {
	/// Create new instance of Serai API.
	pub fn new(client: Arc<Client>) -> Self {
		Serai { client, _phantom: PhantomData }
	}
}

impl<Block, Client> SeraiApiServer for Serai<Block, Client>
where
	Block: BlockT + 'static,
	Client: HeaderBackend<Block> + ProvideRuntimeApi<Block> + Send + Sync + 'static,
	Client::Api: NetworkAddress<Block>,
{
	fn network_address(&self, network: String) -> RpcResult<String> {
		let api = self.client.runtime_api();
		let best_block_hash = self.client.info().best_hash;

		let res = api
			.network_address(best_block_hash, network.as_bytes().to_vec())
			.map_err(|_| Error::Custom(String::from("client error")))?
			.ok_or(Error::Custom(String::from("invalid network")))?;

		let address = String::from_utf8(res).unwrap();
		Ok(address)
	}
}
