const { Leveldb } = require(".")

const db = Leveldb.open("db")

const key = "TheEnd"

const bytes = Array.from(Buffer.from(key))

const data = db.get(bytes)

console.log(data)