(module
    (func $foo)
    (func
        i32.const 0
        call_indirect 
    )
    (table 1 funcref)
    (elem (i32.const 0) $foo)
)