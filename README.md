# pagers
magnolia per page backup service

## Build
```
alias rust-musl-builder='docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder'
rust-musl-builder cargo build --release
scp target/x86_64-unknown-linux-musl/release/pagers <destination>
```

## Environment
* ARCHIVE_DIR=/mnt/<path_to_nfs>/v2
* PREVIOUS_EXT=$(date -d yesterday +%Y%m%d)
* ARCHIVE_EXT=$(date +%Y%m%d)
* BACKUP_URLS='https://<usr:pwd>@<host1>:<port>/<path>,https://<usr:pwd>@<host2>:<port>/<path>,...'

## Notes
Currently this service is only being used by the DAM.  It utilizes our custom exports.jsp code as Magnolia has moved to vaadin to manage the import/export tools which really are only accessible via a browser.  It access Magnolia's RESTful interface to gather a list of leaf nodes / assets that need to be downloaded, exports them individually, and updates the files modify time to match the last_modifed associated with the asset.  Upon a second pass pagers will only export an asset if the last_modified time no longer matches the exported file's modify timestamp.  If it does match then a hard link to the original is made for that days backup.
