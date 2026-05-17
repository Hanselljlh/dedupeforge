# Example commands

## Exact scan

```bash
cargo run --release --bin dedupeforge -- /data/photos
```

## Protect an archive folder

```bash
cargo run --release --bin dedupeforge -- /data/current /data/archive --protected /data/archive
```

## Use XXH3 128-bit

```bash
cargo run --release --bin dedupeforge -- /data/photos --hash xxh3-128
```

## Use SHA-256

```bash
cargo run --release --bin dedupeforge -- /data/photos --hash sha256
```

## Verify matches byte-by-byte

```bash
cargo run --release --bin dedupeforge -- /data/photos --byte-verify
```

## Ignore tiny files

```bash
cargo run --release --bin dedupeforge -- /data/photos --min-size 1048576
```

## Export JSON

```bash
cargo run --release --bin dedupeforge -- /data/photos --output json > duplicates.json
```

## Export CSV

```bash
cargo run --release --bin dedupeforge -- /data/photos --output csv > duplicates.csv
```
