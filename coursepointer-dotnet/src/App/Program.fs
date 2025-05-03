open System
open System.IO

open Dynastream.Fit

type DisposableEncoder(encoder: Encode) =
    member this.Encoder = encoder
    
    interface IDisposable with
        member this.Dispose() = this.Encoder.Close()

let writeTestCourse path =
    use stream = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.Read)
    use wrapper = new DisposableEncoder(new Encode(ProtocolVersion.V10))
    
    wrapper |> _.Encoder |> fun encoder -> encoder.Open(stream)

[<EntryPoint>]
let main _ =
    printfn "Hello, world!"
    writeTestCourse("out.fit")
    0
