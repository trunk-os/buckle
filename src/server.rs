use capnp::capability::Promise;

#[allow(unused)]
struct Server;

impl crate::capnp::buckle::Server for Server {
    fn ping(
        &mut self,
        _: crate::capnp::buckle::PingParams,
        _: crate::capnp::buckle::PingResults,
    ) -> Promise<(), capnp::Error> {
        Promise::ok(())
    }
}
