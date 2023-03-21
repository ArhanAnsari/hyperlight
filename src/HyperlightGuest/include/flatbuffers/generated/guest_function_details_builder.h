#ifndef GUEST_FUNCTION_DETAILS_BUILDER_H
#define GUEST_FUNCTION_DETAILS_BUILDER_H

/* Generated by flatcc 0.6.2 FlatBuffers schema compiler for C by dvide.com */

#ifndef GUEST_FUNCTION_DETAILS_READER_H
#include "guest_function_details_reader.h"
#endif
#ifndef FLATBUFFERS_COMMON_BUILDER_H
#include "flatbuffers_common_builder.h"
#endif
#ifndef GUEST_FUNCTION_DEFINITION_BUILDER_H
#include "guest_function_definition_builder.h"
#endif
#include "flatcc/flatcc_prologue.h"
#ifndef flatbuffers_identifier
#define flatbuffers_identifier 0
#endif
#ifndef flatbuffers_extension
#define flatbuffers_extension "bin"
#endif

static const flatbuffers_voffset_t __Hyperlight_Generated_GuestFunctionDetails_required[] = { 0, 0 };
typedef flatbuffers_ref_t Hyperlight_Generated_GuestFunctionDetails_ref_t;
static Hyperlight_Generated_GuestFunctionDetails_ref_t Hyperlight_Generated_GuestFunctionDetails_clone(flatbuffers_builder_t *B, Hyperlight_Generated_GuestFunctionDetails_table_t t);
__flatbuffers_build_table(flatbuffers_, Hyperlight_Generated_GuestFunctionDetails, 1)

#define __Hyperlight_Generated_GuestFunctionDetails_formal_args , Hyperlight_Generated_GuestFunctionDefinition_vec_ref_t v0
#define __Hyperlight_Generated_GuestFunctionDetails_call_args , v0
static inline Hyperlight_Generated_GuestFunctionDetails_ref_t Hyperlight_Generated_GuestFunctionDetails_create(flatbuffers_builder_t *B __Hyperlight_Generated_GuestFunctionDetails_formal_args);
__flatbuffers_build_table_prolog(flatbuffers_, Hyperlight_Generated_GuestFunctionDetails, Hyperlight_Generated_GuestFunctionDetails_file_identifier, Hyperlight_Generated_GuestFunctionDetails_type_identifier)

/* vector has keyed elements */
__flatbuffers_build_table_vector_field(0, flatbuffers_, Hyperlight_Generated_GuestFunctionDetails_functions, Hyperlight_Generated_GuestFunctionDefinition, Hyperlight_Generated_GuestFunctionDetails)

static inline Hyperlight_Generated_GuestFunctionDetails_ref_t Hyperlight_Generated_GuestFunctionDetails_create(flatbuffers_builder_t *B __Hyperlight_Generated_GuestFunctionDetails_formal_args)
{
    if (Hyperlight_Generated_GuestFunctionDetails_start(B)
        || Hyperlight_Generated_GuestFunctionDetails_functions_add(B, v0)) {
        return 0;
    }
    return Hyperlight_Generated_GuestFunctionDetails_end(B);
}

static Hyperlight_Generated_GuestFunctionDetails_ref_t Hyperlight_Generated_GuestFunctionDetails_clone(flatbuffers_builder_t *B, Hyperlight_Generated_GuestFunctionDetails_table_t t)
{
    __flatbuffers_memoize_begin(B, t);
    if (Hyperlight_Generated_GuestFunctionDetails_start(B)
        || Hyperlight_Generated_GuestFunctionDetails_functions_pick(B, t)) {
        return 0;
    }
    __flatbuffers_memoize_end(B, t, Hyperlight_Generated_GuestFunctionDetails_end(B));
}

#include "flatcc/flatcc_epilogue.h"
#endif /* GUEST_FUNCTION_DETAILS_BUILDER_H */
