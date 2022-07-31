# Rapid web prototyping framework

This is a framework made to quickly build interactive web applications in Rust.

The root crate is the implementation of the framework. The framework itself can be found in `/framework` and `/framework/derive`

**THIS IS A PROOF OF CONCEPT AND NOT READY FOR PRODUCTION**

# Features

- Generates HTML structures based on your data structs:
  - Labels (default)
  - Tables (add `#[table]`, then multiple `#[column(field = "name", header = "Name")]`)
  - TODO:
    - Forms
- Supports automatic reloading through [knockout](https://knockoutjs.com/)

# TODO:

Adding an `#[action(...)]` to your data structure should add a link. This link will then call a function you define. e.g.:

```rust
#[derive(Default, Page, Serialize, Deserialize)]
#[action(name = "Say hi!", fn = "say_hi")]
struct Foo;

impl Foo {
    async fn say_hi(&self, _: Request<()>) -> Result {
        println!("Hello!")
    }
}
```