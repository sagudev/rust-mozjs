use ptr::{null, addr_of};
use result::{Result, Ok, Err};
use libc::{c_char, c_uint};
use name_pool::{name_pool, add};
use str::raw::from_c_str;
use io::WriterUtil;
use jsapi::{JSBool, JSClass, JSContext, JSErrorReport, JSFunctionSpec,
               JSObject, JSRuntime, JSString, JSVERSION_LATEST, jsval,
               JSPropertySpec, JSPropertyOp, JSStrictPropertyOp, JSProto_LIMIT};
use jsapi::bindgen::{JS_free, JS_AddObjectRoot, JS_DefineFunctions,
                        JS_DestroyContext, JS_EncodeString, JS_EvaluateScript,
                        JS_Finish, JS_GetContextPrivate, JS_GetPrivate,
                        JS_Init, JS_InitStandardClasses,
                        JS_NewGlobalObject, JS_NewContext,
                        JS_RemoveObjectRoot, JS_SetContextPrivate,
                        JS_SetErrorReporter, JS_SetOptions, JS_SetPrivate,
                        JS_SetVersion, JS_ValueToString, JS_DefineProperties,
                        JS_DefineProperty, JS_NewObject, JS_ComputeThis};
use libc::types::common::c99::{int8_t, int16_t, int32_t, int64_t, uint8_t,
                                  uint16_t, uint32_t, uint64_t};
use glue::bindgen::{RUST_JSVAL_TO_OBJECT, RUST_JSVAL_IS_PRIMITIVE};
use rust::jsobj;

export JSOPTION_STRICT;
export JSOPTION_WERROR;
export JSOPTION_VAROBJFIX;
export JSOPTION_METHODJIT;

export JSPROP_ENUMERATE;
export JSPROP_SHARED;
export JSPROP_NATIVE_ACCESSORS;

export JSCLASS_GLOBAL_FLAGS;
export JSCLASS_HAS_RESERVED_SLOTS;

export rust;
export name_pool;

export jsapi;
export global;
export glue;
export crust;

export ptr_methods;

// These are just macros in jsapi.h
use JS_NewRuntime = jsapi::bindgen::JS_Init;
export JS_NewRuntime;
use JS_DestroyRuntime = jsapi::bindgen::JS_Finish;
export JS_DestroyRuntime;
/*
FIXME: Not sure where JS_Lock is
use JS_LockRuntime = jsapi::bindgen::JS_Lock;
export JS_LockRuntime;
use JS_UnlockRuntime = jsapi::bindgen::JS_Unlock;
export JS_UnlockRuntime;
*/

export JS_ARGV;
export JS_SET_RVAL;
export JS_THIS_OBJECT;
export JSVAL_VOID;
export JSVAL_NULL;
export JSVAL_ZERO;
export JSVAL_ONE;
export JSVAL_FALSE;
export JSVAL_TRUE;

// FIXME: Add the remaining options
const JSOPTION_STRICT: uint32_t =    0b00000000000001u32;
const JSOPTION_WERROR: uint32_t =    0b00000000000010u32;
const JSOPTION_VAROBJFIX: uint32_t = 0b00000000000100u32;
//const JSOPTION_METHODJIT: uint32_t = 0b100000000000000u32;
const JSOPTION_METHODJIT: uint32_t = (1 << 14) as u32;

const default_heapsize: u32 = 8_u32 * 1024_u32 * 1024_u32;
const default_stacksize: uint = 8192u;
const ERR: JSBool = 0_i32;

const JSVAL_TAG_MAX_DOUBLE: u64 = 0x1FFF0;

const JSVAL_TYPE_DOUBLE: u64 = 0x00;
const JSVAL_TYPE_INT32: u64 = 0x01;
const JSVAL_TYPE_UNDEFINED: u64 = 0x02;
const JSVAL_TYPE_BOOLEAN: u64 = 0x03;
const JSVAL_TYPE_MAGIC: u64 = 0x04;
const JSVAL_TYPE_STRING: u64 = 0x05;
const JSVAL_TYPE_NULL: u64 = 0x06;
const JSVAL_TYPE_OBJECT: u64 = 0x07;
const JSVAL_TYPE_UNKNOWN: u64 = 0x20;

const JSVAL_TAG_SHIFT: int = 47;

//The following constants are totally broken on non-64bit platforms.
//See jsapi.h for the proper macro definitions.
const JSVAL_VOID: u64 = (JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_UNKNOWN) << JSVAL_TAG_SHIFT;
const JSVAL_NULL: u64 = (JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_NULL) << JSVAL_TAG_SHIFT;
const JSVAL_ZERO: u64 = (JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_INT32) << JSVAL_TAG_SHIFT;
const JSVAL_ONE: u64 = ((JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_INT32) << JSVAL_TAG_SHIFT) | 1;
const JSVAL_FALSE: u64 = (JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_BOOLEAN) << JSVAL_TAG_SHIFT;
const JSVAL_TRUE: u64 = ((JSVAL_TAG_MAX_DOUBLE | JSVAL_TYPE_BOOLEAN) << JSVAL_TAG_SHIFT) | 1;

const JSPROP_ENUMERATE: c_uint = 0x01;
const JSPROP_READONLY: c_uint  = 0x02;
const JSPROP_SHARED: c_uint =    0x40;
const JSPROP_NATIVE_ACCESSORS: c_uint = 0x08;

const JSCLASS_RESERVED_SLOTS_SHIFT: c_uint = 8;
const JSCLASS_RESERVED_SLOTS_WIDTH: c_uint = 8;
const JSCLASS_RESERVED_SLOTS_MASK: c_uint = ((1 << JSCLASS_RESERVED_SLOTS_WIDTH) - 1);

const JSCLASS_HIGH_FLAGS_SHIFT: c_uint =
    JSCLASS_RESERVED_SLOTS_SHIFT + JSCLASS_RESERVED_SLOTS_WIDTH;
const JSCLASS_IS_GLOBAL: c_uint = (1<<(JSCLASS_HIGH_FLAGS_SHIFT+1));

const JSCLASS_GLOBAL_SLOT_COUNT: c_uint = JSProto_LIMIT * 3 + 24;

pure fn JSCLASS_HAS_RESERVED_SLOTS(n: c_uint) -> c_uint {
    (n & JSCLASS_RESERVED_SLOTS_MASK) << JSCLASS_RESERVED_SLOTS_SHIFT
}

fn result(n: JSBool) -> Result<(),()> {
    if n != ERR {Ok(())} else {Err(())}
}
fn result_obj(o: jsobj) -> Result<jsobj, ()> {
    if o.ptr != null() {Ok(o)} else {Err(())}
}

type named_functions = @{
    names: ~[~str],
    funcs: ~[JSFunctionSpec]
};

#[inline(always)]
unsafe fn JS_ARGV(_cx: *JSContext, vp: *jsval) -> *jsval {
    ptr::offset(vp, 2u)
}

unsafe fn JS_SET_RVAL(_cx: *JSContext, vp: *jsval, v: jsval) {
    let vp: *mut jsval = cast::reinterpret_cast(&vp);
    *vp = v;
}

#[inline(always)]
unsafe fn JS_THIS_OBJECT(cx: *JSContext, vp: *jsval) -> *JSObject unsafe {
    let r = RUST_JSVAL_TO_OBJECT(
        if RUST_JSVAL_IS_PRIMITIVE(*ptr::offset(vp, 1)) == 0 {
            JS_ComputeThis(cx, vp)
        } else {
            *ptr::offset(vp, 1)
        });
    r
}

