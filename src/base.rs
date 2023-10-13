pub trait ResourceID {
    fn resource_id() -> &'static str;
}

resource_id!(u32, "base::u32");

macro_rules! resource_id {
    ($type:ty, $resource_name:literal) => {
        impl crate::base::ResourceID for $type {
            fn resource_id() -> &'static str {
                $resource_name
            }
        }
    };
}

pub(crate) use resource_id;
