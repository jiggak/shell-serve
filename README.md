shell-serve is a simple web server that routes requests to shell scripts.
It doesn't have to be a shell script; the server uses standard I/O, process
arguments, and environment variables as an interface to the HTTP messages.

## How does it work?

Define your route, such as `POST:/files/{filename} ./handler.sh ${filename}`.
The server will respond to post requests with the URI `/files/{filename}` where
it will _capture_ the path component named `filename` and pass it as an
argument to `./handler.sh`. The standard input stream for `./handler.sh`
contains the request body. Content written to standard output by `./handler.sh`
is streamed to the response body.

The status code of the response is "200 OK" if the `./handler.sh` process exits
with success status (zero), or "500 Internal Server Error" if the process exits
with nonzero. The `./handler.sh` environment variable named `SHELL_SERVE_PIPE`
defines a named pipe for the process to write response headers. If the process
writes a "Status" header, this overrides the exit status code.

```bash
echo "Status: 404" >${SHELL_SERVE_PIPE}
```

## Route definitions

`[METHOD]:[PATH]?[QUERY]#[HEADERS] [HANDLER] <ARGS...>`

* `METHOD` *: any valid HTTP method
* `PATH` *: [path part](#route-path-parts) of the URI
* `QUERY`: query parameters (follows [query part](#route-query-parts) rules)
* `HEADERS`: headers (follows [query part](#route-query-parts) rules)
* `HANDLER` *: route handler command and optional arguments

\* required

## Route path parts

Route path parts are separated by `/` and can be optionally captured and passed
as arguments to the handler. For example the route path `/foo/bar` will match
requests where the URI path is exactly `/foo/bar`.

If the route path is defined as `/foo/{file}`, URI paths such as `/foo/bar` or
`/foo/baz` will match, and `bar` or `baz` is captured and available as a named
parameter to the handler `./handler.sh ${file}`.

Append an ellipsis `..` to the name to specify the path part should match more
than one path component. For example, `/foo/{file..}` will match the URL path
`/foo/bar/baz`, and `bar/baz` is captured in an argument named `${file}`.

Append an asterisk `*` to the name to specify the path part is optional.
For example, `/foo/{file*}` will match the URL path `/foo` or `/foo/bar`.

## Route query parts

Route query parameters and headers are separated by `&` and follow similar rules
as [route path part](#route-path-parts).

* Literal name=value pair, e.g `page=1`
* Name equals some captured value `page={page_number}`
* Optional parameter that equals some captured value `page={page_number*}`
* Capture zero or more `name=value` pairs ("catch all")
  `page={page_number}&{other_args..}`

## Examples

Match any `PUT` request URI and echo the request body to the response body.

```bash
shell-serve 'PUT:/{path..} cat'
```

---

Get the contents of a file in the directory `/tmp/data`

```bash
shell-serve 'GET:/{filename} cat /tmp/data/${filename}'
```

---

Capture query parameter and header and pass to `./handler.sh`

```bash
shell-serve 'GET:/?page={page}#x-auth-token={token} ./handler.sh ${page} ${token}'
```

---

Capture all remaining path components

```bash
shell-serve 'GET:/files/{path..} cat /tmp/data/${path}'
```

---

Simple "todo" server with list/create/delete endpoints

[todo_server.sh](examples/todo_server.sh)


## Configuration File

If your routes become complicated, you might prefer to define them with a
configuration file. Routes are defined either using the same _expression_
format described [above](#route-definitions), or by separating the pieces
of the expression into an object/dictionary style.

```toml
port = 8080
listen = "127.0.0.1"

routes = [
   # "GET:/{path..}?{query..} ./foo.sh ${path} ${query}",
   # This "long form" is equivalent to the definition in the previous line
   { method = "GET", path = "/{path..}?{query..}", handler = "./foo.sh ${path} ${query}"},
   "PUT:/{path..} cat"
]
```