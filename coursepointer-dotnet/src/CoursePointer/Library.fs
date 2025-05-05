namespace CoursePointer

open FSharp.Data.UnitSystems.SI.UnitSymbols
open System
open System.IO

open Dynastream

open Geodesy
open Measures

exception CourseWriterException of string

type CourseWriter(path: string, name: string, startTime: DateTime, speed: float<km/h>) =
    let speed = speed * mPerKm / sPerH
    
    let stream = new FileStream(path, FileMode.Create, FileAccess.ReadWrite, FileShare.Read)
    let encoder = new Fit.Encode(Fit.ProtocolVersion.V10)
    
    let mutable firstPoint = None
    let mutable prevPoint = None
    let mutable totalDistance = 0.0<m>
    
    let writeFileIdMesg() =
        let mesg = new Fit.FileIdMesg()
        mesg.SetType(Fit.File.Course)
        mesg.SetManufacturer(Fit.Manufacturer.Development)
        mesg.SetProduct(0x0001us)
        mesg.SetSerialNumber(0x0001u)
        encoder.Write(mesg)
        
    let writeCourseMesg() =
        let mesg = new Fit.CourseMesg()
        mesg.SetSport(Fit.Sport.Cycling)
        mesg.SetName(name)
        encoder.Write(mesg)
        
    let writeTimerEventMesg(eventType: Fit.EventType) =
        let mesg = new Fit.EventMesg()
        mesg.SetEvent(Fit.Event.Timer)
        mesg.SetEventType(eventType)
        let timestamp = startTime.AddSeconds(float (totalDistance / speed))
        mesg.SetTimestamp(new Fit.DateTime(timestamp))
        encoder.Write(mesg)
        
    let writeLapMesg() =
        let mesg = new Fit.LapMesg()
        mesg.SetStartTime(new Fit.DateTime(startTime))
        mesg.SetTotalElapsedTime(float32 (totalDistance / speed))
        mesg.SetTotalTimerTime(float32 (totalDistance / speed))
        mesg.SetTotalDistance(float32 totalDistance)
        match firstPoint with
        | Some point ->
            mesg.SetStartPositionLat(int (semicircles point.Lat))
            mesg.SetStartPositionLong(int (semicircles point.Lon))
        | None -> ()
        match prevPoint with
        | Some point ->
            mesg.SetEndPositionLat(int (semicircles point.Lat))
            mesg.SetEndPositionLong(int (semicircles point.Lon))
        | None -> ()
        encoder.Write(mesg)
    
    do (
        encoder.Open(stream)
        writeFileIdMesg()
        writeCourseMesg()
        writeTimerEventMesg(Fit.EventType.Start)
        )
    
    member self.AddRecord(point: SurfacePoint) =
        totalDistance <- match prevPoint with
                         | Some previous -> totalDistance + getDistance previous point
                         | None -> 0.0<m>
        if firstPoint.IsNone then
            firstPoint <- Some point
        prevPoint <- Some point
        let timestamp = startTime.AddSeconds(float (totalDistance / speed))
        
        let mesg = new Fit.RecordMesg()
        mesg.SetPositionLat(int (semicircles point.Lat))
        mesg.SetPositionLong(int (semicircles point.Lon))
        mesg.SetTimestamp(new Fit.DateTime(timestamp))
        mesg.SetDistance(float32 totalDistance)
        encoder.Write(mesg)
    
    interface IDisposable with
        member this.Dispose() =
            writeTimerEventMesg(Fit.EventType.Stop)
            writeLapMesg()
            
            encoder.Close()
            stream.Close()
