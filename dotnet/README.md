# CoursePointer

## Development

### Dependency Management

This solution uses [Paket](https://github.com/fsprojects/Paket) for dependency management because at the time of
writing, I couldn't figure out how to use NuGet's central package management to block a transitive dependency on an
older version of FSharp.Core. Paket additionally allows for a full lockfile capturing the versions of transitive
dependencies.

Paket configures the projects with a solution-wide restore file, so from a fresh checkout you should be able to run:

```
dotnet test
```

to build and test the project. To update packages, however, you'll need to use the paket tool:

```
dotnet tool restore
dotnet paket update
```
