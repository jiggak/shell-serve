```bash
shell-serve
   --route 'GET:/foo/{file}=handler_get_foo.sh ${file}' \
   --route 'GET:/{path..}=handler_get.sh ${path}' \
   --route 'PUT:/{path..}=handler_write.sh ${path}' \
   --route 'DELETE:/{path..}=rm some_dir/${path}'
```

GET:/{path..}=handler.sh ${path}
* stdout => response body
* exit status 0 is 200 OK, anything else if 500 Error

curl -i http://localhost:8000/foo/baz.txt
curl -i http://localhost:8000/foo/baz.txt --upload-file hello.txt

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
