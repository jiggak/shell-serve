```bash
shell-serve
   --route 'GET:/foo/{file}?foo={bar} handler_get_foo.sh ${file} ${foo}' \
   --route 'GET:/{path..}?{query..} handler_get.sh ${path} ${query}' \
   --route 'PUT:/{path..} handler_write.sh ${path}' \
   --route 'DELETE:/{path..} rm some_dir/${path}'
```

`GET:/{path..}?poo=blah&foo={bar}&{query..} handler.sh ${path} ${bar} ${query}`

```bash
curl -i http://localhost:8000/foo/baz.txt
curl -i http://localhost:8000/foo/baz.txt --upload-file hello.txt
```

GET
* Req body: no
* Resp body: yes (required?)
* Status: 200

PUT
* Req body: yes
* Resp body: yes (optional)
* Status: 200 (204 resp empty)

POST
* Req body: yes
* Resp body: yes (optional)
* Status: 200 (204 resp empty)

PATCH
* Req body: yes
* Resp body: yes (optional)
* Status: 200 (204 resp empty)

DELETE
* Req body: no
* Resp body: yes (optional)
* Status: 200
