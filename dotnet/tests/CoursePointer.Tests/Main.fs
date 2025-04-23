open Expecto

[<Tests>]
let tests =
    test "An example test" {
        let subject = "Hello world"
        Expect.equal "Hello world" subject "The strings should be equal"
        
        let sum = CoursePointer.Say.addSome 2 3
        Expect.equal sum 5 "2+3 should equal 5"
    }

[<EntryPoint>]
let main args =
    runTestsWithCLIArgs [] args tests
