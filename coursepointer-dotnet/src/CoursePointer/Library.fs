namespace CoursePointer

open FSharp.Data.UnitSystems.SI.UnitSymbols
open System
open System.IO

open Dynastream

open Geodesy
open Measures

type CourseWriter(path: string, name: string, startTime: DateTime, speed: float<km/hr>) =
    let speed = speed * mPerKm / sPerHr
    
    let stream = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.Read)
    let encoder = new Fit.Encode(Fit.ProtocolVersion.V10)
    
    let mutable firstPoint = None
    let mutable prevPoint = None
    let mutable totalDistance = 0.0<m>
    
    let writeFileId() =
        let fileId = new Fit.FileIdMesg()
        fileId.SetType(Fit.File.Course)
        fileId.SetManufacturer(Fit.Manufacturer.Development)
        fileId.SetProduct(0x0001us)
        fileId.SetSerialNumber(0x0001u)
        encoder.Write(fileId)
        
    let writeCourseMesg() =
        let course = new Fit.CourseMesg()
        course.SetSport(Fit.Sport.Cycling)
        course.SetName(name)
        encoder.Write(course)
        
    let writeTimerEvent(eventType: Fit.EventType) =
        let event = new Fit.EventMesg()
        event.SetEvent(Fit.Event.Timer)
        event.SetEventType(eventType)
        let timestamp = startTime.AddSeconds(float (totalDistance / speed))
        event.SetTimestamp(new Fit.DateTime(timestamp))
        encoder.Write(event)
        
    let writeLapMesg() =
        let lap = new Fit.LapMesg()
        lap.SetStartTime(new Fit.DateTime(startTime))
        lap.SetTotalElapsedTime(float32 (totalDistance / speed))
        lap.SetTotalTimerTime(float32 (totalDistance / speed))
        lap.SetTotalDistance(float32 totalDistance)
        match firstPoint with
        | Some point ->
            lap.SetStartPositionLat(int (semicircles point.Lat))
            lap.SetStartPositionLong(int (semicircles point.Lon))
        | None -> ()
        match prevPoint with
        | Some point ->
            lap.SetEndPositionLat(int (semicircles point.Lat))
            lap.SetEndPositionLong(int (semicircles point.Lon))
        | None -> ()
        encoder.Write(lap)
    
    do (
        encoder.Open(stream)
        writeFileId()
        writeCourseMesg()
        writeTimerEvent(Fit.EventType.Start)
        )
    
    member self.AddRecord(point: SurfacePoint) =
        totalDistance <- match prevPoint with
                         | Some previous -> totalDistance + getDistance previous point
                         | None -> 0.0<m>
        if firstPoint.IsNone then
            firstPoint <- Some point
        prevPoint <- Some point
        let timestamp = startTime.AddSeconds(float (totalDistance / speed))
        
        let record = new Fit.RecordMesg()
        record.SetPositionLat(int (semicircles point.Lat))
        record.SetPositionLong(int (semicircles point.Lon))
        record.SetTimestamp(new Fit.DateTime(timestamp))
        record.SetDistance(float32 totalDistance)
        encoder.Write(record)
    
    interface IDisposable with
        member this.Dispose() =
            writeTimerEvent(Fit.EventType.Stop)
            writeLapMesg()
            
            encoder.Close()
            stream.Close()
