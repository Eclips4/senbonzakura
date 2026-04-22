(function_def body: (_) @function.inside) @function.around

(data_decl body: (_) @class.inside) @class.around

(typeclass_decl body: (_) @class.inside) @class.around

(impl_block body: (_) @class.inside) @class.around

(parameter) @parameter.inside

(comment) @comment.inside @comment.around
