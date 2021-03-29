use thruster::{App, BasicContext,Request ,async_middleware};
use super::super::plaintext;

pub fn get_app() -> App<Request, BasicContext, ()>{
    let mut public = App::<Request, BasicContext, ()>::new_basic();
    public.get("/news", async_middleware!(BasicContext, [plaintext]));
    return public
}