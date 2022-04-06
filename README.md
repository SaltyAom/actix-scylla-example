## Actix Scylla Example
Example of using Actix Web with ScyllaDB in Rust.

To setup development server, simply [run Scylla using Docker](https://rust-driver.docs.scylladb.com/stable/quickstart/scylla-docker.html)
```bash
docker run --rm -it -p 9042:9042 scylladb/scylla --smp 2
```