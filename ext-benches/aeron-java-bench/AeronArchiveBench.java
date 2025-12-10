///usr/bin/env jbang "$0" "$@" ; exit $?
//DEPS io.aeron:aeron-all:1.47.3
//JAVA_OPTIONS --add-opens java.base/jdk.internal.misc=ALL-UNNAMED
//JAVA_OPTIONS --add-opens java.base/java.nio=ALL-UNNAMED
//JAVA_OPTIONS -Dagrona.disable.bounds.checks=true

import io.aeron.Aeron;
import io.aeron.Publication;
import io.aeron.archive.Archive;
import io.aeron.archive.ArchiveThreadingMode;
import io.aeron.archive.client.AeronArchive;
import io.aeron.archive.codecs.SourceLocation;
import io.aeron.driver.MediaDriver;
import io.aeron.driver.ThreadingMode;
import org.agrona.concurrent.UnsafeBuffer;

import java.io.File;
import java.nio.file.Files;

/**
 * Aeron Archive Benchmark - measures archive throughput
 * Run with: jbang AeronArchiveBench.java
 */
public class AeronArchiveBench {
    private static final int MESSAGE_COUNT = 500_000;  // Same as kaos-rudp
    private static final int MESSAGE_SIZE = 64;
    private static final String CHANNEL = "aeron:ipc";
    private static final int STREAM_ID = 1001;

    public static void main(String[] args) throws Exception {
        System.out.println("\n╔═══════════════════════════════════════════╗");
        System.out.println("║  Aeron Archive Benchmark                  ║");
        System.out.println("╚═══════════════════════════════════════════╝");
        System.out.printf("%nConfig: %d msgs, %d bytes%n%n", MESSAGE_COUNT, MESSAGE_SIZE);

        // Create temp directories
        File aeronDir = Files.createTempDirectory("aeron-bench").toFile();
        File archiveDir = Files.createTempDirectory("aeron-archive").toFile();

        aeronDir.deleteOnExit();
        archiveDir.deleteOnExit();

        // Start Media Driver
        MediaDriver.Context driverCtx = new MediaDriver.Context()
            .aeronDirectoryName(aeronDir.getAbsolutePath())
            .threadingMode(ThreadingMode.SHARED)
            .dirDeleteOnStart(true)
            .dirDeleteOnShutdown(true);

        // Start Archive
        Archive.Context archiveCtx = new Archive.Context()
            .aeronDirectoryName(aeronDir.getAbsolutePath())
            .archiveDir(archiveDir)
            .controlChannel("aeron:udp?endpoint=localhost:8010")
            .replicationChannel("aeron:udp?endpoint=localhost:0")
            .threadingMode(ArchiveThreadingMode.SHARED)
            .deleteArchiveOnStart(true);

        try (MediaDriver driver = MediaDriver.launch(driverCtx);
             Archive archive = Archive.launch(archiveCtx)) {

            AeronArchive.Context archiveClientCtx = new AeronArchive.Context()
                .aeronDirectoryName(aeronDir.getAbsolutePath())
                .controlRequestChannel("aeron:udp?endpoint=localhost:8010")
                .controlResponseChannel("aeron:udp?endpoint=localhost:0");

            try (AeronArchive aeronArchive = AeronArchive.connect(archiveClientCtx)) {

                aeronArchive.startRecording(CHANNEL, STREAM_ID, SourceLocation.LOCAL);

                try (Publication publication = aeronArchive.context().aeron()
                        .addPublication(CHANNEL, STREAM_ID)) {

                    while (!publication.isConnected()) {
                        Thread.yield();
                    }

                    UnsafeBuffer buffer = new UnsafeBuffer(new byte[MESSAGE_SIZE]);

                    // Warmup
                    System.out.println("Warming up...");
                    for (int i = 0; i < 50_000; i++) {
                        while (publication.offer(buffer, 0, MESSAGE_SIZE) < 0) {
                            Thread.yield();
                        }
                    }
                    Thread.sleep(200);

                    // Benchmark
                    System.out.println("Benchmarking...");
                    long start = System.nanoTime();

                    for (int i = 0; i < MESSAGE_COUNT; i++) {
                        while (publication.offer(buffer, 0, MESSAGE_SIZE) < 0) {
                            Thread.yield();
                        }
                    }

                    long sendTime = System.nanoTime() - start;
                    Thread.sleep(500); // wait for archive
                    long totalTime = System.nanoTime() - start;

                    double sendSec = sendTime / 1e9;
                    double totalSec = totalTime / 1e9;

                    System.out.println("\n═══════════════════════════════════════════");
                    System.out.printf("  Messages:    %d%n", MESSAGE_COUNT);
                    System.out.printf("  Send time:   %.3fs%n", sendSec);
                    System.out.printf("  Throughput:  %.2f M/s%n", MESSAGE_COUNT / sendSec / 1e6);
                    System.out.printf("  Latency:     %.1f ns/msg%n", (double) sendTime / MESSAGE_COUNT);
                    System.out.println("═══════════════════════════════════════════\n");
                }

                aeronArchive.stopRecording(CHANNEL, STREAM_ID);
            }
        }
    }
}
