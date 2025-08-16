use mime::Mime;
use std::fmt;
use std::path::Path;
use std::sync::LazyLock;

macro_rules! create_mime_from_str {
    (
        $(
            $value:expr => $mime:ident : $mime_name:literal
        ),+ $(,)?
    ) => {
        $(
            pub static $mime: LazyLock<Mime> = LazyLock::new(|| {
                $value
                    .parse()
                    .expect(concat!($mime_name, " should be a valid mime type"))
            });
        )+
    };
}

pub const MIME_TYPE_DRIVE_FOLDER: &str = "application/vnd.google-apps.folder";
pub const MIME_TYPE_DRIVE_DOCUMENT: &str = "application/vnd.google-apps.document";
pub const MIME_TYPE_DRIVE_SHORTCUT: &str = "application/vnd.google-apps.shortcut";
pub const MIME_TYPE_DRIVE_SPREADSHEET: &str = "application/vnd.google-apps.spreadsheet";
pub const MIME_TYPE_DRIVE_PRESENTATION: &str = "application/vnd.google-apps.presentation";

create_mime_from_str!(
    MIME_TYPE_DRIVE_DOCUMENT => MIME_TYPE_DRIVE_DOCUMENT_MIME: "drive document" ,
    MIME_TYPE_DRIVE_SPREADSHEET => MIME_TYPE_DRIVE_SPREADSHEET_MIME: "drive spreadsheet",
    MIME_TYPE_DRIVE_PRESENTATION => MIME_TYPE_DRIVE_PRESENTATION_MIME: "drive presentation",
);

pub const EXTENSION_DOC: &str = "doc";
pub const EXTENSION_DOCX: &str = "docx";
pub const EXTENSION_ODT: &str = "odt";
pub const EXTENSION_JPG: &str = "jpg";
pub const EXTENSION_JPEG: &str = "jpeg";
pub const EXTENSION_GIF: &str = "gif";
pub const EXTENSION_PNG: &str = "png";
pub const EXTENSION_RTF: &str = "rtf";
pub const EXTENSION_PDF: &str = "pdf";
pub const EXTENSION_HTML: &str = "html";
pub const EXTENSION_XLS: &str = "xls";
pub const EXTENSION_XLSX: &str = "xlsx";
pub const EXTENSION_CSV: &str = "csv";
pub const EXTENSION_TSV: &str = "tsv";
pub const EXTENSION_ODS: &str = "ods";
pub const EXTENSION_PPT: &str = "ppt";
pub const EXTENSION_PPTX: &str = "pptx";
pub const EXTENSION_ODP: &str = "odp";
pub const EXTENSION_EPUB: &str = "epub";
pub const EXTENSION_TXT: &str = "txt";

pub const MIME_TYPE_DOC: &str = "application/msword";
pub const MIME_TYPE_DOCX: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const MIME_TYPE_ODT: &str = "application/vnd.oasis.opendocument.text";
pub const MIME_TYPE_JPG: &str = "image/jpeg";
pub const MIME_TYPE_JPEG: &str = "image/jpeg";
pub const MIME_TYPE_GIF: &str = "image/gif";
pub const MIME_TYPE_PNG: &str = "image/png";
pub const MIME_TYPE_RTF: &str = "application/rtf";
pub const MIME_TYPE_PDF: &str = "application/pdf";
pub const MIME_TYPE_HTML: &str = "text/html";
pub const MIME_TYPE_XLS: &str = "application/vnd.ms-excel";
pub const MIME_TYPE_XLSX: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const MIME_TYPE_CSV: &str = "text/csv";
pub const MIME_TYPE_TSV: &str = "text/tab-separated-values";
pub const MIME_TYPE_ODS: &str = "application/vnd.oasis.opendocument.spreadsheet";
pub const MIME_TYPE_PPT: &str = "application/vnd.ms-powerpoint";
pub const MIME_TYPE_PPTX: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub const MIME_TYPE_ODP: &str = "application/vnd.oasis.opendocument.presentation";
pub const MIME_TYPE_EPUB: &str = "application/epub+zip";
pub const MIME_TYPE_TXT: &str = "text/plain";

create_mime_from_str!(
    MIME_TYPE_DOC => MIME_TYPE_DOC_MIME: "microsoft doc",
    MIME_TYPE_DOCX => MIME_TYPE_DOCX_MIME: "microsoft docx",
    MIME_TYPE_ODT => MIME_TYPE_ODT_MIME: "opendocument text",
    MIME_TYPE_JPG => MIME_TYPE_JPG_MIME: "jpeg image",
    MIME_TYPE_JPEG => MIME_TYPE_JPEG_MIME: "jpeg image",
    MIME_TYPE_GIF => MIME_TYPE_GIF_MIME: "gif image",
    MIME_TYPE_PNG => MIME_TYPE_PNG_MIME: "png image",
    MIME_TYPE_RTF => MIME_TYPE_RTF_MIME: "rich-text format",
    MIME_TYPE_PDF => MIME_TYPE_PDF_MIME: "pdf document",
    MIME_TYPE_HTML => MIME_TYPE_HTML_MIME: "html document",
    MIME_TYPE_XLS => MIME_TYPE_XLS_MIME: "microsoft xls",
    MIME_TYPE_XLSX => MIME_TYPE_XLSX_MIME: "microsoft xlsx",
    MIME_TYPE_CSV => MIME_TYPE_CSV_MIME: "comma separated file",
    MIME_TYPE_TSV => MIME_TYPE_TSV_MIME: "tab separated file",
    MIME_TYPE_ODS => MIME_TYPE_ODS_MIME: "opendocument spreadsheet",
    MIME_TYPE_PPT => MIME_TYPE_PPT_MIME: "microsoft ppt",
    MIME_TYPE_PPTX => MIME_TYPE_PPTX_MIME: "microsoft pptx",
    MIME_TYPE_ODP => MIME_TYPE_ODP_MIME: "opendocument presentation",
    MIME_TYPE_EPUB => MIME_TYPE_EPUB_MIME: "epub document",
    MIME_TYPE_TXT => MIME_TYPE_TXT_MIME: "plain text",
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocType {
    Document,
    Spreadsheet,
    Presentation,
}

impl DocType {
    const IMPORT_EXTENSION_MAP: [(FileExtension, DocType); 18] = [
        (FileExtension::Doc, DocType::Document),
        (FileExtension::Docx, DocType::Document),
        (FileExtension::Odt, DocType::Document),
        (FileExtension::Jpg, DocType::Document),
        (FileExtension::Jpeg, DocType::Document),
        (FileExtension::Gif, DocType::Document),
        (FileExtension::Png, DocType::Document),
        (FileExtension::Rtf, DocType::Document),
        (FileExtension::Pdf, DocType::Document),
        (FileExtension::Html, DocType::Document),
        (FileExtension::Xls, DocType::Spreadsheet),
        (FileExtension::Xlsx, DocType::Spreadsheet),
        (FileExtension::Csv, DocType::Spreadsheet),
        (FileExtension::Tsv, DocType::Spreadsheet),
        (FileExtension::Ods, DocType::Spreadsheet),
        (FileExtension::Ppt, DocType::Presentation),
        (FileExtension::Pptx, DocType::Presentation),
        (FileExtension::Odp, DocType::Presentation),
    ];

    pub const SUPPORTED_INPUT_TYPES: [FileExtension; Self::IMPORT_EXTENSION_MAP.len()] = const {
        let mut out = [FileExtension::Doc; Self::IMPORT_EXTENSION_MAP.len()];
        let mut index = 0;
        while index < Self::IMPORT_EXTENSION_MAP.len() {
            out[index] = Self::IMPORT_EXTENSION_MAP[index].0;
            index += 1;
        }
        out
    };

    #[must_use]
    pub fn from_file_path(path: &Path) -> Option<DocType> {
        let extension = FileExtension::from_path(path)?;

        Self::IMPORT_EXTENSION_MAP
            .iter()
            .find_map(|&(ext, doc_type)| {
                if ext == extension {
                    Some(doc_type)
                } else {
                    None
                }
            })
    }

    #[must_use]
    pub fn from_mime_type(mime: &str) -> Option<DocType> {
        match mime {
            MIME_TYPE_DRIVE_DOCUMENT => Some(DocType::Document),
            MIME_TYPE_DRIVE_SPREADSHEET => Some(DocType::Spreadsheet),
            MIME_TYPE_DRIVE_PRESENTATION => Some(DocType::Presentation),
            _ => None,
        }
    }

    #[must_use]
    pub fn default_export_type(&self) -> FileExtension {
        match self {
            DocType::Spreadsheet => FileExtension::Csv,
            DocType::Presentation | DocType::Document => FileExtension::Pdf,
        }
    }

    #[must_use]
    pub fn can_export_to(&self, extension: FileExtension) -> bool {
        self.supported_export_types().contains(&extension)
    }

    #[must_use]
    pub fn supported_export_types(&self) -> &'static [FileExtension] {
        match self {
            DocType::Document => &[
                FileExtension::Pdf,
                FileExtension::Odt,
                FileExtension::Docx,
                FileExtension::Epub,
                FileExtension::Rtf,
                FileExtension::Txt,
                FileExtension::Html,
            ],

            DocType::Spreadsheet => &[
                FileExtension::Csv,
                FileExtension::Tsv,
                FileExtension::Ods,
                FileExtension::Xlsx,
                FileExtension::Pdf,
            ],

            DocType::Presentation => &[
                FileExtension::Pdf,
                FileExtension::Pptx,
                FileExtension::Odp,
                FileExtension::Txt,
            ],
        }
    }

    #[must_use]
    pub fn mime(&self) -> &'static Mime {
        match self {
            DocType::Document => &MIME_TYPE_DRIVE_DOCUMENT_MIME,
            DocType::Spreadsheet => &MIME_TYPE_DRIVE_SPREADSHEET_MIME,
            DocType::Presentation => &MIME_TYPE_DRIVE_PRESENTATION_MIME,
        }
    }
}

const _: () = const {
    let mut i = 0;
    while i < DocType::IMPORT_EXTENSION_MAP.len() {
        let extension_i = DocType::IMPORT_EXTENSION_MAP[i].0;

        let mut j = i + 1;
        while j < DocType::IMPORT_EXTENSION_MAP.len() {
            let extension_j = DocType::IMPORT_EXTENSION_MAP[j].0;
            assert!(
                !extension_i.eq_const(extension_j),
                "IMPORT_EXTENSION_MAP cannot contain duplicated file extensions"
            );

            j += 1;
        }

        i += 1;
    }
};

impl fmt::Display for DocType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DocType::Document => write!(f, "document"),
            DocType::Spreadsheet => write!(f, "spreadsheet"),
            DocType::Presentation => write!(f, "presentation"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtension {
    Doc,
    Docx,
    Odt,
    Jpg,
    Jpeg,
    Gif,
    Png,
    Rtf,
    Pdf,
    Html,
    Xls,
    Xlsx,
    Csv,
    Tsv,
    Ods,
    Ppt,
    Pptx,
    Odp,
    Epub,
    Txt,
}

impl FileExtension {
    const fn eq_const(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Doc, Self::Doc)
                | (Self::Docx, Self::Docx)
                | (Self::Odt, Self::Odt)
                | (Self::Jpg, Self::Jpg)
                | (Self::Jpeg, Self::Jpeg)
                | (Self::Gif, Self::Gif)
                | (Self::Png, Self::Png)
                | (Self::Rtf, Self::Rtf)
                | (Self::Pdf, Self::Pdf)
                | (Self::Html, Self::Html)
                | (Self::Xls, Self::Xls)
                | (Self::Xlsx, Self::Xlsx)
                | (Self::Csv, Self::Csv)
                | (Self::Tsv, Self::Tsv)
                | (Self::Ods, Self::Ods)
                | (Self::Ppt, Self::Ppt)
                | (Self::Pptx, Self::Pptx)
                | (Self::Odp, Self::Odp)
                | (Self::Epub, Self::Epub)
                | (Self::Txt, Self::Txt)
        )
    }
}

impl fmt::Display for FileExtension {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileExtension::Doc => write!(f, "{EXTENSION_DOC}"),
            FileExtension::Docx => write!(f, "{EXTENSION_DOCX}"),
            FileExtension::Odt => write!(f, "{EXTENSION_ODT}"),
            FileExtension::Jpg => write!(f, "{EXTENSION_JPG}"),
            FileExtension::Jpeg => write!(f, "{EXTENSION_JPEG}"),
            FileExtension::Gif => write!(f, "{EXTENSION_GIF}"),
            FileExtension::Png => write!(f, "{EXTENSION_PNG}"),
            FileExtension::Rtf => write!(f, "{EXTENSION_RTF}"),
            FileExtension::Pdf => write!(f, "{EXTENSION_PDF}"),
            FileExtension::Html => write!(f, "{EXTENSION_HTML}"),
            FileExtension::Xls => write!(f, "{EXTENSION_XLS}"),
            FileExtension::Xlsx => write!(f, "{EXTENSION_XLSX}"),
            FileExtension::Csv => write!(f, "{EXTENSION_CSV}"),
            FileExtension::Tsv => write!(f, "{EXTENSION_TSV}"),
            FileExtension::Ods => write!(f, "{EXTENSION_ODS}"),
            FileExtension::Ppt => write!(f, "{EXTENSION_PPT}"),
            FileExtension::Pptx => write!(f, "{EXTENSION_PPTX}"),
            FileExtension::Odp => write!(f, "{EXTENSION_ODP}"),
            FileExtension::Epub => write!(f, "{EXTENSION_EPUB}"),
            FileExtension::Txt => write!(f, "{EXTENSION_TXT}"),
        }
    }
}

impl FileExtension {
    #[must_use]
    pub fn from_path(path: &Path) -> Option<FileExtension> {
        let extension = path.extension()?.to_str()?;

        match extension {
            EXTENSION_DOC => Some(FileExtension::Doc),
            EXTENSION_DOCX => Some(FileExtension::Docx),
            EXTENSION_ODT => Some(FileExtension::Odt),
            EXTENSION_JPG => Some(FileExtension::Jpg),
            EXTENSION_JPEG => Some(FileExtension::Jpeg),
            EXTENSION_GIF => Some(FileExtension::Gif),
            EXTENSION_PNG => Some(FileExtension::Png),
            EXTENSION_RTF => Some(FileExtension::Rtf),
            EXTENSION_PDF => Some(FileExtension::Pdf),
            EXTENSION_HTML => Some(FileExtension::Html),
            EXTENSION_XLS => Some(FileExtension::Xls),
            EXTENSION_XLSX => Some(FileExtension::Xlsx),
            EXTENSION_CSV => Some(FileExtension::Csv),
            EXTENSION_TSV => Some(FileExtension::Tsv),
            EXTENSION_ODS => Some(FileExtension::Ods),
            EXTENSION_PPT => Some(FileExtension::Ppt),
            EXTENSION_PPTX => Some(FileExtension::Pptx),
            EXTENSION_ODP => Some(FileExtension::Odp),
            EXTENSION_EPUB => Some(FileExtension::Epub),
            EXTENSION_TXT => Some(FileExtension::Txt),
            _ => None,
        }
    }

    #[must_use]
    pub fn get_export_mime(&self) -> &'static Mime {
        match self {
            FileExtension::Doc => &MIME_TYPE_DOC_MIME,
            FileExtension::Docx => &MIME_TYPE_DOCX_MIME,
            FileExtension::Odt => &MIME_TYPE_ODT_MIME,
            FileExtension::Jpg => &MIME_TYPE_JPG_MIME,
            FileExtension::Jpeg => &MIME_TYPE_JPEG_MIME,
            FileExtension::Gif => &MIME_TYPE_GIF_MIME,
            FileExtension::Png => &MIME_TYPE_PNG_MIME,
            FileExtension::Rtf => &MIME_TYPE_RTF_MIME,
            FileExtension::Pdf => &MIME_TYPE_PDF_MIME,
            FileExtension::Html => &MIME_TYPE_HTML_MIME,
            FileExtension::Xls => &MIME_TYPE_XLS_MIME,
            FileExtension::Xlsx => &MIME_TYPE_XLSX_MIME,
            FileExtension::Csv => &MIME_TYPE_CSV_MIME,
            FileExtension::Tsv => &MIME_TYPE_TSV_MIME,
            FileExtension::Ods => &MIME_TYPE_ODS_MIME,
            FileExtension::Ppt => &MIME_TYPE_PPT_MIME,
            FileExtension::Pptx => &MIME_TYPE_PPTX_MIME,
            FileExtension::Odp => &MIME_TYPE_ODP_MIME,
            FileExtension::Epub => &MIME_TYPE_EPUB_MIME,
            FileExtension::Txt => &MIME_TYPE_TXT_MIME,
        }
    }
}

#[must_use]
pub fn is_directory(file: &google_drive3::api::File) -> bool {
    file.mime_type == Some(String::from(MIME_TYPE_DRIVE_FOLDER))
}

#[must_use]
pub fn is_binary(file: &google_drive3::api::File) -> bool {
    file.md5_checksum.is_some()
}

#[must_use]
pub fn is_shortcut(file: &google_drive3::api::File) -> bool {
    file.mime_type == Some(String::from(MIME_TYPE_DRIVE_SHORTCUT))
}
