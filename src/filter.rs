use solana_program::pubkey::Pubkey;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::collections::HashSet;
use thiserror::Error;
use regex::Regex;
use bs58;
use hex;

use solana_snapshot_etl::append_vec::StoredAccountMeta;

#[derive(Error, Debug)]
pub enum FilterParseError {
  #[error("Invalid owner filter syntax")]
  InvalidOwnerFilterSyntax,
  #[error("Invalid owner pubkey")]
  InvalidOwnerPubkey,

  #[error("Invalid size filter")]
  InvalidSizeFilter,
  #[error("Multiple size filter")]
  MultipleSizeFilter,

  #[error("Invalid memcmp filter (bytes)")]
  InvalidBytesMemcmpFilter,
  #[error("Invalid memcmp filter (offset)")]
  InvalidOffsetMemcmpFilter,

  #[error("Unknown filter")]
  UnknownFilter,
}

pub struct MemCmp {
  offset: usize,
  bytes: Vec<u8>,
}

pub struct OwnerFilter {
  owner: Pubkey,
  size_filter: Option<u64>,
  memcmp_filters: Vec<MemCmp>,
}

pub struct AccountFilter {
  pubkey_filters: HashSet<String>,
  owner_filters: Vec<OwnerFilter>,
}

impl MemCmp {
  pub fn is_match(&self, data: &[u8]) -> bool {
    if self.offset + self.bytes.len() > data.len() { return false; }

    for i in 0..self.bytes.len() {
      if data[self.offset+i] != self.bytes[i] { return false; }
    }

    return true;
  }
}

impl OwnerFilter {
  pub fn new(owner_with_opts: &String) -> Result<Self, FilterParseError> {
    let re_owner_filter = Regex::new(r"^([abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ123456789]+)((?:,[^,]+)*)$").unwrap();
    let re_size_filter = Regex::new(r"^size:(\d+)$").unwrap();
    let re_memcmp_hex_filter = Regex::new(r"memcmp:0x((?:[0-9a-fA-F][0-9a-fA-F])+)@(\d+)$").unwrap();
    let re_memcmp_base58_filter = Regex::new(r"memcmp:([abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ123456789]+)@(\d+)$").unwrap();

    if !re_owner_filter.is_match(&owner_with_opts) {
      return Err(FilterParseError::InvalidOwnerFilterSyntax);
    }

    let caps = re_owner_filter.captures(&owner_with_opts).unwrap();
    let owner_base58 = &caps[1];
    let opts = &caps[2];

    let owner = Pubkey::from_str(owner_base58).or_else(|_e| Err(FilterParseError::InvalidOwnerPubkey))?;

    let mut size_filter: Option<u64> = None;
    let mut memcmp_filters: Vec<MemCmp> = vec![];
    for opt in opts.split(',') {
      if opt.is_empty() { continue; }

      if re_size_filter.is_match(opt) {
        let caps = re_size_filter.captures(opt).unwrap();
        let size = caps[1].parse::<u64>().or_else(|_e| Err(FilterParseError::InvalidSizeFilter))?;
        match size_filter {
          Some(_size) => { return Err(FilterParseError::MultipleSizeFilter); }
          None => { size_filter = Some(size) },
        }
      }
      else if re_memcmp_hex_filter.is_match(opt) {
        let caps = re_memcmp_hex_filter.captures(opt).unwrap();
        let bytes = hex::decode(&caps[1]).or_else(|_e| Err(FilterParseError::InvalidBytesMemcmpFilter))?;
        let offset = caps[2].parse::<usize>().or_else(|_e| Err(FilterParseError::InvalidOffsetMemcmpFilter))?;
        memcmp_filters.push(MemCmp { bytes, offset });
      }
      else if re_memcmp_base58_filter.is_match(opt) {
        let caps = re_memcmp_base58_filter.captures(opt).unwrap();
        let bytes = bs58::decode(&caps[1]).into_vec().or_else(|_e| Err(FilterParseError::InvalidBytesMemcmpFilter))?;
        let offset = caps[2].parse::<usize>().or_else(|_e| Err(FilterParseError::InvalidOffsetMemcmpFilter))?;
        memcmp_filters.push(MemCmp { offset, bytes });
      }
      else {
        return Err(FilterParseError::UnknownFilter);
      }
    }

    Ok(OwnerFilter {
      owner,
      size_filter,
      memcmp_filters,
    })
  }

  pub fn is_match(&self, account: &StoredAccountMeta) -> bool {
    match self.size_filter {
      Some(size) => {
        if account.meta.data_len != size { return false; }
      }
      None => {}
    }

    if !account.account_meta.owner.eq(&self.owner) { return false; }

    for memcmp in self.memcmp_filters.iter() {
      if !memcmp.is_match(account.data) { return false; }
    }

    return true;
  }
}

impl AccountFilter {
  pub fn new(pubkeys: &Vec<String>, pubkeyfile: &Option<String>, owners: &Vec<String>) -> Result<Self, FilterParseError> {
    let mut pubkey_filters: HashSet<String> = HashSet::new();
    let mut owner_filters: Vec<OwnerFilter> = vec![];

    // --pubkey=pk1
    // --pubkey=pk1,pk2,pk3,...
    for pubkey in pubkeys.iter() {
      for pk in pubkey.split(',') {
        pubkey_filters.insert(pk.to_string());
      }
    }

    // --pubkeyfile=file (1 pubkey per line)
    match pubkeyfile {
      None => {},
      Some(file) => {
        let f = File::open(file).unwrap();
        let reader = BufReader::new(f);
        for line in reader.lines() {
          let line = line.unwrap();
          let trimed = line.trim();
          if trimed.len() == 0 { continue }

          pubkey_filters.insert(trimed.to_string());
        }
      },
    }

    // --owner=TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
    // --owner=TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA,size:165
    // --owner=TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA,size:82,memcmp:0x06@44
    for owner in owners.iter() {
      let owner_filter = OwnerFilter::new(owner)?;
      owner_filters.push(owner_filter);
    }

    Ok(AccountFilter {
      pubkey_filters,
      owner_filters,
    })
  }


  pub fn is_match(&self, account: &StoredAccountMeta) -> bool {
    if self.pubkey_filters.is_empty() && self.owner_filters.is_empty() { return true; }

    if self.pubkey_filters.contains(&account.meta.pubkey.to_string()) { return true; }

    for owner_filter in self.owner_filters.iter() {
      if owner_filter.is_match(account) { return true; }
    }

    return false;
  }
}