//! Windows 模型包选择、校验、安装和本地资源定位；不上传 Windows 路径给 Linux 语音引擎。

mod model;

pub use model::{
    AssetError, InstalledModel, ModelImportLimits, ValidatedModelPackage, install_model_package,
    validate_model_package,
};
