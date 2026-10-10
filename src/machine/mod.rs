//! Native object model, collection format, connection geometry, and rendering.
//! Public exports preserve the original machine API; implementation lives in
//! focused sibling modules. Object/list order is retained by the collection codec.
use crate::definition::PartDefinition;
use crate::resources::Resources;
mod update_flag;
pub use update_flag::UpdateFlag;
mod property_flag;
pub use property_flag::PropertyFlag;
#[path = "../tables/render_tables.rs"]
mod render_tables;
mod rope_connection;
pub use rope_connection::*;
mod object_list;
pub use object_list::*;
mod render_flag;
pub use render_flag::RenderFlag;
mod render_flags;
pub use render_flags::*;
mod contact_record;
pub use contact_record::*;
mod object;
pub use object::*;
mod collection;
pub use collection::*;
mod connections;
pub use connections::*;
mod render;
pub use render::*;
mod geometry;
pub use geometry::*;

#[cfg(test)]
mod save_tests;

mod reader;
use reader::Reader;
mod collection_metadata;
pub use collection_metadata::CollectionMetadata;

mod rope_span;
pub(crate) use rope_span::RopeSpan;
