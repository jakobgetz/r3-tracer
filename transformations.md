# Transformations

## store
```wasm
global.get $mem_pointer
i32.const ;; opcode
i32.store8 offset=0
global.get $mem_pointer
local.tee $addr
i32.store offset=1
global.get $mem_pointer
local.tee $value
xxx.storex offset=5
global.get $mem_pointer
i32.const ;; 5 + load byte length
i32.add
global.set $mem_pointer
local.get $addr
local.get $value
;; original_store
```

## store
```wasm
global.get $mem_pointer
i32.const 0x36
i32.store8 $trace_mem offset=0
local.set 0 ;; save value to local
global.get $mem_pointer
local.get 0
i64.store16 $trace_mem offset=1
local.tee 1 ;; store addr to local
global.get $mem_pointer
local.get 1
i64.store16 $trace_mem offset=5
local.get 0
i64.store16
global.get $mem_pointer ;; increment mem_pointer
i32.const 7
i32.add
global.set $mem_pointer
```

## load
```wasm
global.get $mem_pointer
i32.const ;; opcode
i32.store8 offset=0
global.get $mem_pointer
local.tee $addr
i32.store offset=1
global.get $mem_pointer
local.get $addr
;; original_load
local.tee $value
xxx.storex offset=5
global.get $mem_pointer
i32.const ;; 5 + load byte length
i32.add
global.set $mem_pointer
local.get $value
```

## function begin
```wasm
global.get $mem_pointer
i32.const ;; code for func begin + type idx
i32.store8 offset=0
global.get $mem_pointer
local.get 0
xxx.storex offset=1
;; ... also store other arguments
global.get $mem_pointer
i32.const ;; 5 + arg byte length
i32.add
global.set $mem_pointer
```

## return
(for every return instruction or when the function block ends)
```wasm
global.get $mem_pointer
i32.const ;; code for func end + type idx
i32.store8 offset=0
local.tee $return_value1
global.get $mem_pointer
xxx.storex offset=1
;; also deal with other return values
global.get $mem_pointer
i32.const ;; 5 + return values byte length
i32.add
global.set $mem_pointer
local.get $return_value1
;; return or func end
```

## call
```wasm
global.get $mem_pointer
i32.const ;; code for call
i32.store8 offset=0
global.get $mem_pointer
i32.const ;; type idx
i32.store offset=1
global.get $mem_pointer
i32.const ;; func idx
i32.store offset=5
local.tee $arg_1
global.get $mem_pointer
i32.store offset=9
;; do the same for other args
global.get $mem_pointer
i32.const ;; args byte length
i32.add
global.set $mem_pointer
;; ... retrieve all args
local.get $arg_1
call ;; func idx
```

## call
```wasm
global.get $mem_pointer
i32.const 0x10
i32.store8 $trace_mem offset=0
global.get $mem_pointer
i32.const 0 ;; type idx
i32.store $trace_mem offset=1
global.get $mem_pointer
i32.const 0 ;; func idx
i32.store $trace_mem offset=5
local.tee 0 ;; save arg local
global.get $mem_pointer
local.get 1
i32.store $trace_mem offset=9
call 0
local.tee 0 ;; save result to local
global.get $mem_pointer
local.get 0
i32.store $trace_mem offset=13
global.get $mem_pointer ;; increment mem_pointer
i32.const 17
i32.add
global.set $mem_pointer

```

## table set
```wasm
global.get $mem_pointer
i32.const ;; opcode
i32.store8 offset=0
global.get $mem_pointer
i32.const ;; table idx
i32.store offset=1
global.get $mem_pointer
local.tee $index_into_table
i32.store offset=5
local.get $index_into_table
table.set ;; table idx
```