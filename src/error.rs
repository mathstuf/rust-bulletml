// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

error_chain! {
    errors {
        NoSuchEntity(name: String) {
            display("could not find entity `{}`", name)
        }

        ExpressionParseFailure {
            display("expression parse failure")
        }

        UndefinedVariable(name: String) {
            display("undefined variable `{}`", name)
        }
    }
}
