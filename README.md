# Tauri Lite
## API
### File System
- [x] stat(path) -> Stat
- [x] exists(path) -> boolean

- [x] read(path) -> string
- [x] write(path, data: string) -> void
- [ ] append(path, data: string) -> void

- [x] mv(path, newPath) -> void
- [x] cp(path, newPath) -> void
- [x] rm(path) -> void

- [x] ls(path) -> string[]
- [x] mkDir(path) -> void
- [x] rmDir(path) -> void

- [ ] link(path, newPath) -> void

### Http
- [x] request(url, options?) -> string
- [ ] download(url, path, options?) -> void

### OS
- [x] info() -> string
- [x] dirs() -> string

### Process
- [x] exec(command | file, args?, options?) -> string
- [x] pid() -> number
- [x] cwd() -> string
- [x] chDir() -> void
- [x] env() -> Env
- [x] exit() -> !