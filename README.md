# @serenityjs/leveldb

This package allows the ease of reading/writing from a [leveldb](https://github.com/google/leveldb) formated database.

This package incorporates the super speeds of [Rust](https://www.rust-lang.org/tools/install) by using the pre-existing package [rusty-leveldb](https://docs.rs/rusty-leveldb/latest/rusty_leveldb/). The package creates [NodeJS](https://nodejs.org/en/) bindings to be used in any Node programs.

## Example Usage
```ts
import { Leveldb } from "@serenityjs/leveldb"

// Open a database with a given path
const db = Leveldb.open("./mydatabase")

// Write a key-value pair to the database
db.put(Buffer.from("Hello"), Buffer.from("World"))

// Read the value for a key from the database
const data = db.get(Buffer.from("Hello"))
const value = data.toString('utf8')

// Delete a key from the database
db.delete(Buffer.from("Hello"))

// Close the database when done!
db.close()

```