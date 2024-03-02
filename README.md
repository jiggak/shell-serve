restfuse
   --route GET:/foo/{file}=handler_get_foo.sh {file}
   --route GET:/{path..}=handler_get.sh {path}
   --route PUT:/{path..}=handler_write.sh {path}
   --route DELETE:/{path..}=rm some_dir/{path}
