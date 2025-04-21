sub add_target {
    $target = $_[0];
    # drop trailing '*'
    foreach my $source (@sources)
    {
        $source =~ s{\*$}{};
    }
    @additional_sources = ();
    print(FH "add_executable(", $target, " ", (join " ", @sources) , " ", (join " ", @additional_sources), ")\n");

    # add portability flags
    $bench_cflags = $bench_cflags . " -std=c99";
    $bench_flags = $bench_flags . " -DSPEC_LP64";

    # CPUFLAGS in Makefile.defaults
    $bench_flags = $bench_flags . " -DSPEC -DNDEBUG";

    # statically linked
    print(FH "target_link_libraries(", $target, " -static)\n");

    # link math library
    print(FH "target_link_libraries(", $target, " m)\n");

    # benchmark specific flags
    if ($target eq "500.perlbench_r") {
        $bench_flags = $bench_flags . " -DSPEC_LINUX_X64";
    }
    if ($target eq "521.wrf_r") {
        $bench_cflags = $bench_cflags . " -DSPEC_CASE_FLAG";
        $bench_fflags = $bench_fflags . " -fconvert=big-endian";
    }
    if ($target eq "523.xalancbmk_r") {
        $bench_flags = $bench_flags . " -DSPEC_LINUX";
    }
    if ($target eq "526.blender_r") {
        $bench_flags = $bench_flags . " -funsigned-char -DSPEC_LINUX";
    }
    if ($target eq "527.cam4_r") {
        $bench_flags = $bench_flags . " -DSPEC_CASE_FLAG";
    }
    if ($target eq "554.roms_r") {
        # fix compilation
        $bench_flags = $bench_flags . " -DNDEBUG";
    }

    # add optimize flags
    $bench_flags = $bench_flags . " -O3";

    # CXXOPTIMIZE
    # for 510.parest_r
    $bench_cxxflags = $bench_cxxflags . " -std=c++03";

    # FOPTIMIZE
    # for 521.wrf_r
    $bench_fflags = $bench_fflags . " -fallow-argument-mismatch";

    # EXTRA_COPTIMIZE
    # for 500.perlbench_r
    $bench_cflags = $bench_cflags . " -fno-strict-aliasing -fno-unsafe-math-optimizations -fno-finite-math-only";
    # for 502.gcc_r
    $bench_cflags = $bench_cflags . " -fgnu89-inline";
    # for 525.x264_r
    $bench_cflags = $bench_cflags . " -fcommon";
    # for 527.cam4_r
    $bench_cflags = $bench_cflags . " -Wno-error=implicit-int";

    # convert -I flags to target_include_directories
    for $flag (split(" ", ($bench_flags . " " . $bench_cflags . " " . $bench_cxxflags . " " . $bench_fflags . " " . $bench_fppflags))) {
        if ((rindex $flag, "-I", 0) == 0) {
            print(FH "target_include_directories(", $target, " PRIVATE ", substr($flag, 2, length($flag)), ")\n");
        }
    }

    # drop unwanted preprocessor flags for specpp
    $bench_fppflags =~ s{-w -m literal-single.pm -m c-comment.pm}{};
    $bench_fppflags =~ s{-w -m literal.pm}{};

    # due to using gfortran as preprocessor, the expanded __FILE__ may exceed column limit
    # add workaround
    $bench_fflags = $bench_fflags . " -ffree-line-length-512";

    # set flags for each language
    $bench_cflags = $bench_cflags . " " . $bench_flags;
    $bench_cxxflags = $bench_cxxflags . " " . $bench_flags;
    $bench_fflags = $bench_fflags . " " . $bench_fppflags . " " . $bench_flags;

    print(FH "target_compile_options(", $target, " PRIVATE\n\t\$<\$<COMPILE_LANGUAGE:C>:", $bench_cflags, ">\n\t\$<\$<COMPILE_LANGUAGE:CXX>:", $bench_cxxflags, ">\n\t\$<\$<COMPILE_LANGUAGE:Fortran>:", $bench_fflags, ">)\n");
}

system("mkdir -p data");
mkdir("src");
open(FH2, '>', "src/CMakeLists.txt") or die $!;
print(FH2 "cmake_minimum_required(VERSION 3.10)\n");
print(FH2 "project(spec2017)\n");
print(FH2 "enable_language(C CXX Fortran)\n");
for $benchmark ("500.perlbench_r", "502.gcc_r", "505.mcf_r", "520.omnetpp_r", "523.xalancbmk_r", "525.x264_r", "531.deepsjeng_r", "541.leela_r", "548.exchange2_r", "557.xz_r", "503.bwaves_r", "507.cactuBSSN_r", "508.namd_r", "510.parest_r", "511.povray_r", "519.lbm_r", "521.wrf_r", "526.blender_r", "527.cam4_r", "538.imagick_r", "544.nab_r", "549.fotonik3d_r", "554.roms_r") {
    $bench_flags = $bench_cflags = $bench_cxxflags = $bench_fflags = $bench_fppflags = "";
    require "./benchspec/CPU/" . $benchmark . "/Spec/object.pm";
    mkdir("src/" . $benchmark);
    system("cp -arv ./benchspec/CPU/" . $benchmark . "/src/* src/" . $benchmark . "/");
    open(FH, '>', "src/" . $benchmark . "/CMakeLists.txt") or die $!;
    if ($benchmark eq "511.povray_r") {
        @sources = @{%sources{"povray_r"}};
        add_target("511.povray_r");
    } elsif ($benchmark eq "521.wrf_r") {
        @sources = @{%sources{"wrf_r"}};
        add_target("521.wrf_r");
    } elsif ($benchmark eq "525.x264_r") {
        @sources = @{%sources{"x264_r"}};
        add_target("525.x264_r");

        @sources = @{%sources{"ldecod_r"}};
        add_target("ldecod_r");
    } elsif ($benchmark eq "526.blender_r") {
        @sources = @{%sources{"blender_r"}};
        add_target("526.blender_r");
    } elsif ($benchmark eq "527.cam4_r") {
        @sources = @{%sources{"cam4_r"}};
        add_target("527.cam4_r");
    } elsif ($benchmark eq "538.imagick_r") {
        @sources = @{%sources{"imagick_r"}};
        add_target("538.imagick_r");
    } else {
        add_target($benchmark);
    }

    print(FH2 "add_subdirectory(" . $benchmark . ")\n");

    if ($benchmark eq "549.fotonik3d_r") {
        # extract OBJ.dat.xz for input
        system("xz -d -k ./benchspec/CPU/549.fotonik3d_r/data/refrate/input/OBJ.dat.xz");
    }

    # collect data
    system("mkdir -p data/" . $benchmark);
    system("cp -rv ./benchspec/CPU/" . $benchmark . "/data/all/input/* ./benchspec/CPU/" . $benchmark . "/data/refrate/input/* " . "data/" . $benchmark . "/");
}

# patch code
# fix compilation
system("sed -i 's/^#ifdef\$/#ifdef SPEC/' src/527.cam4_r/ESMF_AlarmMod.F90");

# build code using ninja
system("mkdir -p build-o3 && cd build-o3 && cmake ../src -G Ninja -DCMAKE_C_FLAGS=\"-g\" -DCMAKE_CXX_FLAGS=\"-g\" -DCMAKE_Fortran_FLAGS=\"-g\" && ninja && cd .. && tar cvzf build-o3.tar.gz build-o3");
system("mkdir -p build-o3-lto && cd build-o3-lto && cmake ../src -G Ninja -DCMAKE_C_FLAGS=\"-g -flto\" -DCMAKE_CXX_FLAGS=\"-g -flto\" -DCMAKE_Fortran_FLAGS=\"-g -flto\" && ninja && cd .. && tar cvzf build-o3-lto.tar.gz build-o3-lto");
