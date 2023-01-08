# Worker Cloud Platform

Worker Cloud is an open-source platform that provides a set of APIs and tools for building and deploying web and mobile applications. It aims to make it easier for developers to create and manage the backend infrastructure for their applications, such as databases, user management, storage, and more.

Worker Cloud includes a range of features, including:

* A set of APIs for building web and mobile applications
* A user management system for authentication and authorization
* A database for storing and querying data
* A file storage system for storing and serving files
* A serverless function platform for running custom logic
* A hosting platform for deploying and hosting applications

Worker Code is built on top of modern technologies such as Docker, Kubernetes, and WebAssembly (WASM) and can be run on-premises or in the cloud. It is designed to be easy to use and to allow developers to focus on building their applications rather than worrying about the underlying infrastructure.
## Installation

If you have rust (cargo) installed, you can build and install the wkr runtime with:

```bash
cargo install wkr-runtime
```

---


We also provide pre-built binaries for **Windows**, **Linux** and **macOS** on the
[releases page][5], that you can include in your `PATH`.

---

And as always, you can also clone this repository and build it locally. The only dependency is
[a rust compiler][7]:

```bash
# Clone the repository
git clone https://github.com/wkr-solutions/wkr.git
# Jump into the cloned folder
cd wkr
# Build and install wkr
cargo install --path .
```

## Usage

After installation, you can use the `wkr` binary to run WASM modules.

To learn how to build modules, check out language-specific bindings:

- [AssemblyScript](https://github.com/worker-codes/workerscript)

## Documentation

- [Getting Started](https://docs.worker.codes/)
- [API Reference](https://docs.worker.codes/api/index.html)
- [Examples](https://github.com/worker-codes/wkr-example)

## Open source

We develop wkr in the open. We're [Apache licensed](https://github.com/worker-codes/workerruntime/blob/main/LICENSEE) and designed to run easily in local dev. You can deploy our core software to production, but it takes a little elbow grease and a fair amount of infrastructure. If you want to give this a try, let us know and we can help (and we would love related pull requests!).

Our commercial offering is built on top of this library, with additional code for managing certificates, distributed caching, and multi-tenant isolation. Over time we expect to extract many of these features, document them, and include them in our open source releases.

[1]: https://github.com/worker-codes/workerscript
[2]: https://github.com/worker-codes/workerscript
[3]: https://github.com/worker-codes/workerscript