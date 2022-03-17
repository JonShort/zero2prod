# zero2prod
Repo following the [Zero To Production In Rust][1] book by [Luca Palmieri][2]

My notes while reading the book can be found in the [docs/][docs/] folder

The source repo for the book can be found [here][2]

## Useful commands

Development loop
```bash
cargo watch -i docs -x check -x test -x run
```

Start local Postgres DB w/ docker
```bash
./scripts/init_db.sh
```

[1]: https://www.zero2prod.com
[2]: https://github.com/LukeMathWalker
[3]: https://github.com/LukeMathWalker/zero-to-production
