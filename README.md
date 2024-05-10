```bash
shell-serve \
   'GET:/{path..}?{query..} ./foo.sh ${path} ${query}' \
   'PUT:/{path..} cat'

shell-serve --listen 127.0.0.1 --port 8080 \
   'GET:/foo/{file}?foo={bar} handle_get_foo.sh ${file} ${foo}' \
   'GET:/poo/{file}#x-auth-token={token} handle_get_poo.sh ${file} ${token}' \
   'GET:/{path..}?{query..} handle_get.sh ${path} ${query}' \
   'PUT:/{path..} handle_write.sh ${path}' \
   'DELETE:/{path..} rm some_dir/${path}'
```


```bash
# test GET
curl --include http://localhost:8000/foo/baz.txt
# test PUT
curl --include http://localhost:8000/echo --upload-file hello.txt
```


```toml
port = 8080
listen = "127.0.0.1"

routes = [
   # "GET:/{path..}?{query..} ./foo.sh ${path} ${query}",
   { method = "GET", path = "/{path..}?{query..}", handler = "./foo.sh ${path} ${query}"},
   "PUT:/{path..} cat"
]
```