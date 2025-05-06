# Garmin FIT Notes

Some notes as I wrap my head around Garmin's FIT file format.

## Reference docs

Relevant to writing a course file:

- [FIT Protocol](https://developer.garmin.com/fit/protocol/) describes the low-level file format
- [FIT Course Files](https://developer.garmin.com/fit/file-types/course/) describes the message types and ordering expected in a course file
- [Encoding Course Files](https://developer.garmin.com/fit/cookbook/encoding-course-files/) provides an example of creating a course file using Garmin’s .NET SDK

## Data types

### Strings

Strings are specified UTF-8 and null-terminated.  This is a bit awkward because they're generally stored in fixed-size fields, both making the null terminator redundant and limiting flexibility.  I wonder why they didn't go with a TLV-style frame format instead.

### Semicircles

A signed (2's complement) 32-bit integer representing a latitude or longitude.

### Timestamps

A 32-bit unsigned number of seconds since the FIT epoch, defined as UTC midnight on 1989-12-31: https://developer.garmin.com/fit/cookbook/datetime/

## Notes on files created by other tools

To make sure we create a broadly compatible FIT course file (and to clarify some ambiguities in Garmin's documentation), we can examine the files created by other tools to see what conventions they actually follow in terms of message types and ordering, local message numbers, endianness, and so on.

### Garmin's .NET SDK

The .NET version seems to be one of Garmin's most complete FIT SDK implementations.  Here I look at the file `out.fit` produced by my F# example code in this repo's commit id 6639488407a03f17be1d5c5091aae447f3830c83, using the Garmin .NET SDK version 21.158.  This isn't totally to spec with Garmin's cookbook example or course file documentation, but it still provides some useful information:

The .NET SDK writes files little endian by default, and sets the header's protocol version byte to 0x10.

The two-byte profile version number, when represented in base 10, corresponds to the SDK's version number: 21158 from SDK version 21.158.

The Garmin docs present this unclearly, but a data definition record header contains developer data definitions iff bit 5 is set in its record header.  The SDK isn't setting this bit by default.

The "global message number" in the definition message corresponds to values in the `mesg_num` area of the Types tab of [Profile.xlsx](https://developer.garmin.com/fit/download/), which can be cross-referenced by name with message definitions in the Messages tab.

For field definitions within a definition record:

- The first byte corresponds to the "Field Def #" column in the Messages tab
- The second byte is a size in bytes
- The third byte corresponds to the "Value" column in the Types tab (maybe specifically in the `fit_base_type` area?)

The definition message for the File ID message is missing some of the fields defined for that message type in the global profile, presumably because I didn't set them in my code.  I'm seeing (values in hex):

| Field Def | Size | Type |
|-----------|------|------|
|         0 |    1 |    0 |
|         1 |    2 |   84 |
|         2 |    2 |   84 |
|         3 |    4 |   8c |

The subsequent File ID message's record header is zeroed out.

In the definition message preceeding the "course" message, the third byte of the field definition for "sport" is zero, which corresponds to the value for "enum" in the `fit_base_type` area of the Types tab.  The definition for "name" has type string, with a specified length of 0x0c.  This happens to be *just* long enough to store the name I'd set in my code, plus its null terminator.  I don't have any course points set in this file, so I can't see how this gets encoded in the case of multiple messages of the same type, with strings of varying lengths.

Notably, this definition message also reuses local message type zero rather than define a new one.  Which I guess is fine, because we're not writing any additional File ID messages.

The first point written in my test program had latitude 52.0 degrees, longitude 13.0 degrees.  The first "record" message has a latitude semicircles value of hexadecimal 24fa4fa5, representing a 32-bit two's complement signed integer of positive decimal value 620384165.  This indeed equals 52.0 * (2^31 / 180).

The longitude value likewise matches this formula for converting degrees into semicircles.

The timestamp field appears to represent a number of seconds since Garmin's epoch of 1989-12-31 00:00:00 UTC.  Interesting choice.

The file CRC is simply presented as two bytes following the final data message in the file, with no additional header.

## Device experiments

TODO
