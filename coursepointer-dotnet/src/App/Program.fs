open System
open System.IO

open Dynastream.Fit

type EncoderWrapper(stream: Stream) =
    let encoder = new Encode(ProtocolVersion.V10)
    do encoder.Open(stream)
    
    member _.Encoder = encoder
    
    interface IDisposable with
        member this.Dispose() = this.Encoder.Close()
        
let writeTestCourse path =
    use stream = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.Read)
    use wrapper = new EncoderWrapper(stream)
    
    wrapper.Encoder |> fun encoder ->
        let fileId = new FileIdMesg()
        fileId.SetType(File.Course)
        fileId.SetManufacturer(Manufacturer.Development)
        fileId.SetProduct(0x0001us)
        fileId.SetSerialNumber(0x0001u)
        encoder.Write(fileId)
        
        let course = new CourseMesg()
        course.SetSport(Sport.Cycling)
        course.SetName("Test Course")
        encoder.Write(course)
        
        let lap = new LapMesg()
        lap.SetStartTime(new Dynastream.Fit.DateTime(DateTime.Now))
        lap.SetTotalElapsedTime(3600.0f)
        encoder.Write(lap)
        
        ()

[<EntryPoint>]
let main _ =
    printfn "Writing test course"
    writeTestCourse("out.fit")
    0
