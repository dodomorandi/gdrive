pub(crate) mod copy;
pub(crate) mod delete;
pub(crate) mod download;
pub(crate) mod export;
pub(crate) mod generate_ids;
pub(crate) mod import;
pub(crate) mod info;
pub(crate) mod list;
pub(crate) mod mkdir;
pub(crate) mod mv;
pub(crate) mod rename;
pub(crate) mod update;
pub(crate) mod upload;

pub(crate) use copy::copy;
pub(crate) use delete::delete;
pub(crate) use download::download;
pub(crate) use export::export;
pub(crate) use generate_ids::generate_ids;
pub(crate) use import::import;
pub(crate) use info::info;
pub(crate) use list::list;
pub(crate) use mkdir::mkdir;
pub(crate) use mv::mv;
pub(crate) use rename::rename;
pub(crate) use update::update;
pub(crate) use upload::upload;

type FileResult = Result<google_drive3::api::File, google_drive3::Error>;

const _: () = {
    #[expect(clippy::manual_assert, reason = "cannot assert in const context")]
    if std::mem::size_of::<google_drive3::api::File>() < std::mem::size_of::<google_drive3::Error>()
    {
        panic!("ok variant smaller than error");
    }
};
