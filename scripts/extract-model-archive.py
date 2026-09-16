"""Extract the curated G2PW archive within the project model library."""
from pathlib import Path, PurePosixPath
import shutil
import stat
import sys
import zipfile


def extract(archive, destination):
    destination = Path(destination).resolve()
    with zipfile.ZipFile(archive) as bundle:
        entries = bundle.infolist()
        if len(entries) > 500 or sum(entry.file_size for entry in entries) > 2 * 1024**3:
            raise ValueError("模型压缩包超过解压限制")
        for entry in entries:
            path = PurePosixPath(entry.filename)
            if (path.is_absolute() or ".." in path.parts or "\\" in entry.filename
                    or not path.parts or path.parts[0] != "G2PWModel"
                    or stat.S_ISLNK(entry.external_attr >> 16)):
                raise ValueError("模型压缩包包含无效路径")
            target = destination.joinpath(*path.parts)
            if not target.resolve().is_relative_to(destination):
                raise ValueError("模型解压路径超出保存目录")
        for entry in entries:
            target = destination.joinpath(*PurePosixPath(entry.filename).parts)
            if entry.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with bundle.open(entry) as source, open(target.with_suffix(target.suffix + ".part"), "wb") as output:
                shutil.copyfileobj(source, output)
            target.with_suffix(target.suffix + ".part").replace(target)


if __name__ == "__main__":
    extract(sys.argv[1], sys.argv[2])
