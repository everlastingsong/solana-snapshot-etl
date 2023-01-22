# Solana Snapshot gPA 🧙

[![license](https://img.shields.io/badge/license-Apache--2.0-blue?style=flat-square)](#license)

**`solana-snapshot-gpa` efficiently extracts specific accounts in a snapshot** to load them into an external system.

## Motivation

Solana nodes periodically backup their account database into a `.tar.zst` "snapshot" stream.
If you run a node yourself, you've probably seen a snapshot file such as this one already:

```
snapshot-139240745-D17vR2iksG5RoLMfTX7i5NwSsr4VpbybuX1eqzesQfu2.tar.zst
```

A full snapshot file contains a copy of all accounts at a specific slot state (in this case slot `139240745`).

Historical accounts data is relevant to blockchain analytics use-cases and event tracing.
Despite archives being readily available, the ecosystem was missing an easy-to-use tool to access snapshot data.

@terorie have created solana-snapshot-etl, and it is great tool.

In using [solana-snapshot-etl](https://github.com/terorie/solana-snapshot-etl),
I found it useful to be able to filter accounts by criteria such as getProgramAccounts and get data for accounts with any pubkey.
solana-snapshot-gpa is a tool for this purpose. solana-snapshot-gpa is based on solana-snapshot-etl.

## Building

```shell
cargo install --git https://github.com/everlastingsong/solana-snapshot-gpa
```

## Usage

The basic command-line usage is as follows:

```
# Extract all accounts
solana-snapshot-gpa snapshot.tar.zst > result.csv

# Extract specific accounts based on pubkeys
solana-snapshot-gpa --pubkey=pubkey1,pubkey2,pubkey3 --pubkey=pubkey4,pubkey5 --pubkey=pubkey6 snapshot.tar.zst > result.csv

# Extract specific accounts based on pubkeys (pubkeys are listed in a text file)
#
# pubkeys.txt (1 pubkey per line)
# pubkey1
# pubkey2
# pubkey3
# 
solana-snapshot-gpa --pubkeyfile=pubkeys.txt snapshot.tar.zst > result.csv

# Extract specific accounts based on owner program with filters
# owner program = TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
# size = 165
# pubkey stored from the 32nd byte is r21Gamwd9DtyjHeGywsneoQYR39C1VDwrw7tWxHAwh6
solana-snapshot-gpa --owner=TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA,size:165,memcmp:r21Gamwd9DtyjHeGywsneoQYR39C1VDwrw7tWxHAwh6@32 snapshot.tar.zst > result.csv

# Extract specific accounts based on owner program with filters
# owner program = whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc
# size = 44
# u16(little endian) stored from the 40th byte is 128
solana-snapshot-gpa --owner=whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc,size:44,memcmp:0x8000@40 snapshot.tar.zst > result.csv
```

### Output (CSV)
* CSV with 7 columns is output.
* The account data is encoded in base64.
* Because of the internal format of the output, called AppendVec, multiple versions of the account with different write_version columns are output. The most recent write_version is appropriate.

```
pubkey,owner,data_len,lamports,write_version,data
HT55NVGVTjWmWLjV7BrSMPVZ7ppU8T2xE5nCAZ6YaGad,whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc,44,1197120,492556471222,OEufTI5EvmkT5EH4ORPKaLBjT7Al/eqohzfoQRDRJV41ezN33e4czUAAuAs=
4kuxsCskbbAvoME1JEdNXJJFWRWP2af2kotyQpmwsVcU,whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc,44,1197120,490748093121,OEufTI5EvmkF3IgGzHvjy10P4GItvoRPV06kXHzKT391zj26u8lAs0AArA0=
```

### Suppliment

#### How to pick up latest write_version

```
# extract to result.csv
solana-snapshot-gpa
 --owner=xxxxxxx
 --owner=xxxxxxx,size:xxxx
 --owner=xxxxxxx,size:xxxx,memcmp:0xffffffff@offset,memcmp:base58@offset
 --pubkey=xxxxxx,xxxxxxxx,xxxxxxx,xxxxxxx,xxxxxxx,xxxxxxxx,xxxxxxx
 --pubkey=xxxxxx,xxxxxxxx,xxxxxxx,xxxxxxx,xxxxxxx,xxxxxxxx,xxxxxxx
 snapshot.tar.zst > result.csv

# pick up latest write_version only
tail -n +2 result.csv | sort -t, -k5,5nr | awk -F, '!dup[$1]++' > result.latest.csv
```

#### How to create JSON to load solana-test-validator

```
# extract to result.csv
solana-snapshot-gpa
 --owner=xxxxxxx
 --owner=xxxxxxx,size:xxxx
 --owner=xxxxxxx,size:xxxx,memcmp:0xffffffff@offset,memcmp:base58@offset
 --pubkey=xxxxxx,xxxxxxxx,xxxxxxx,xxxxxxx,xxxxxxx,xxxxxxxx,xxxxxxx
 --pubkey=xxxxxx,xxxxxxxx,xxxxxxx,xxxxxxx,xxxxxxx,xxxxxxxx,xxxxxxx
 snapshot.tar.zst > result.csv

# pick up latest write_version only
tail -n +2 result.csv | sort -t, -k5,5nr | awk -F, '!dup[$1]++' > result.latest.csv

# conver to JSON (directory: accounts)
mkdir accounts
cat result.latest.csv | awk -F, -v out="accounts" '{ filename=out"/"$1".json"; print "{\"pubkey\":\"" $1 "\",\"account\":{\"lamports\":" $4 ",\"data\":[\"" $6 "\",\"base64\"],\"owner\":\"" $2 "\",\"executable\":false,\"rentEpoch\":0}}" > filename; close(filename) }'

# startup solana-test-validator with extracted accounts
solana-test-validator --account-dir accounts --reset 
```
