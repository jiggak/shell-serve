#!/bin/bash

DATA_DIR=/tmp/todo_data

if [ ! -f ${DATA_DIR} ]; then
   mkdir ${DATA_DIR}
fi

function get_new_file_name() {
   cnt=1
   while true; do
      file_name="todo${cnt}.txt"
      if [ -f "${DATA_DIR}/${file_name}" ]; then
         cnt=$(( $cnt + 1 ))
      else
         echo ${file_name}
         break
      fi
   done
}

function list_todo() {
   ls ${DATA_DIR} | jq --raw-input --slurp --compact-output '{items: split("\n")[:-1]}'
}

function get_todo() {
   local file_name=${1}
   if [ -f ${DATA_DIR}/${file_name} ]; then
      cat ${DATA_DIR}/${file_name}
   else
      echo "Status: 404" >${SHELL_SERVE_PIPE}
   fi
}

function create_todo() {
   local file_name=${1}
   if [ -z "${file_name}" ]; then
      file_name=$(get_new_file_name)
   fi

   cat >${DATA_DIR}/${file_name}
}

function remove_todo() {
   local file_name=${1}
   if [ -f ${DATA_DIR}/${file_name} ]; then
      rm ${DATA_DIR}/${file_name}
   else
      echo "Status: 404" >${SHELL_SERVE_PIPE}
   fi
}

function start() {
   shell-serve \
      'GET:/ ./todo_server.sh list_todo' \
      'GET:/{file_name} ./todo_server.sh get_todo ${file_name}' \
      'PUT:/{file_name*} ./todo_server.sh create_todo ${file_name}' \
      'DELETE:/{file_name} ./todo_server.sh remove_todo ${file_name}'
}

cmd=$1
shift

case $cmd in
   start) start ;;
   get_todo) get_todo $@;;
   list_todo) list_todo $@;;
   create_todo) create_todo $@;;
   remove_todo) remove_todo $@;;
esac
