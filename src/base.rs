pub trait ResourceID {
    fn resource_id() -> &'static str;
}

impl ResourceID for u32 {
    fn resource_id() -> &'static str {
        "base::u32"
    }
}
