open Expecto

[<Tests>]
let tests =
    test "An example test" {
        let subject = "Hello world"
        Expect.equal "Hello world" subject "The strings should be equal"
    }

[<EntryPoint>]
let main args =
    runTestsWithCLIArgs [] args tests
