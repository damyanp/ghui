<script lang="ts" module>
  import dayjs from "dayjs";

  export type Data = {
    epics: Epic[];
    startDate: Date;
  };

  export type Date = string;

  export type Epic = {
    name: string;
    targetDate: Date;
    scenarios: Scenario[];
  };

  export type Scenario = {
    name: string;
    rows: Row[];
  };

  export type Row = {
    bars: Bar[];
  };

  export type Bar = {
    state: BarState;
    label?: string;
    start: Date;
    end: Date;
  };

  export type BarState =
    | "completed"
    | "onTrack"
    | "atRisk"
    | "offTrack"
    | "notStarted"
    | "noDates";
</script>

<script lang="ts">
  import { ZoomIn, ZoomOut } from "@lucide/svelte";
  import {
    ExecutionTrackerContext,
    getExecutionTrackerContext,
    setExecutionTrackerContext,
  } from "./ExecutionTrackerContext.svelte";
  import type { Attachment } from "svelte/attachments";
  import type { Snippet } from "svelte";

  type Props = {
    data: Data;
    scenarioEditor?: Snippet<[Scenario]>;
    barEditor?: Snippet<[Bar]>;
  };

  let { data, scenarioEditor, barEditor }: Props = $props();

  let context =
    getExecutionTrackerContext() ||
    setExecutionTrackerContext(new ExecutionTrackerContext());

  const [minDate, maxDate] = $derived.by(() => {
    let minDate = Number.MAX_VALUE;
    let maxDate = Number.MIN_VALUE;

    for (const epic of data.epics) {
      for (const scenario of epic.scenarios) {
        for (const row of scenario.rows) {
          for (const bar of row.bars) {
            if (bar.start) {
              minDate = Math.min(minDate, dayjs(bar.start).unix());
              maxDate = Math.max(maxDate, dayjs(bar.start).unix());
            }
            if (bar.end) {
              minDate = Math.min(minDate, dayjs(bar.end).unix());
              maxDate = Math.max(maxDate, dayjs(bar.end).unix());
            }
          }
        }
      }
    }

    return [minDate, maxDate];
  });

  const [minX, maxX] = $derived([
    minDate * context.scale,
    maxDate * context.scale,
  ]);

  function convertDate(date: string): number {
    return dayjs(date).unix() * context.scale;
  }

  function getBarFillStyle(state: BarState): string {
    switch (state) {
      case "atRisk":
        return "background-color: #f7c7ac;";
      case "completed":
        return "background-color: #c0e6f5;";
      case "noDates":
        return "background-color: #d9d9d9; background-size: 4px 4px; background-image: linear-gradient(45deg, #9d9d9d 2px, transparent 0);";
      case "notStarted":
        return "background-color: #d9d9d9;";
      case "offTrack":
        return "background-color: #ff7c80;";
      case "onTrack":
        return "background-color: #c1f0c8;";
    }
  }

  function getEpicRowSpan(epic: Epic) {
    return Math.max(
      1,
      epic.scenarios.reduce((prev, current) => {
        return prev + Math.max(1, current.rows.length);
      }, 0)
    );
  }

  const totalRows = $derived(
    data.epics.reduce((prev, current) => {
      return prev + getEpicRowSpan(current);
    }, 0)
  );

  const chartWidth = $derived(maxX - minX);

  const dates = $derived.by(() => {
    let dates = [];

    let date = dayjs(data.startDate);
    let endDate = dayjs.unix(maxDate).add(1, "day");

    while (date < endDate) {
      dates.push({
        value: date.unix() * context.scale - minX,
        label: date.format("MM-DD"),
      });
      date = date.add(7, "day");
    }

    return dates;
  });

  function getEpicFillStyle(index: number) {
    if (index % 2 == 0) {
      return "background-color: #202020;";
    } else {
      return "background-color: #203030;";
    }
  }

  let startDragPos: [[number, number], [number, number]] | undefined =
    $state(undefined);
  const scrollableId = $props.id();

  const pan: Attachment<HTMLElement> = (element: HTMLElement) => {
    element.addEventListener("pointerdown", onScrollPointerDown);
    element.addEventListener("pointerup", onScrollPointerUp);
    element.addEventListener("pointermove", onScrollPointerMove);

    $effect(() => {
      if (startDragPos) {
        element.classList.remove("cursor-grab");
        element.classList.add("cursor-grabbing");
      } else {
        element.classList.add("cursor-grab");
        element.classList.remove("cursor-grabbing");
      }
    });

    return () => {
      element.removeEventListener("pointerdown", onScrollPointerDown);
      element.removeEventListener("pointerup", onScrollPointerUp);
      element.removeEventListener("pointermove", onScrollPointerMove);
    };
  };

  function onScrollPointerDown(e: PointerEvent) {
    if (e.button === 0) {
      const scrollable = document.getElementById(scrollableId)!;
      startDragPos = [
        [e.screenX, e.screenY],
        [scrollable.scrollLeft, scrollable.scrollTop],
      ];
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.preventDefault();
    }
  }

  function onScrollPointerUp(e: PointerEvent) {
    if (e.button === 0) {
      startDragPos = undefined;
      (e.target as HTMLElement).releasePointerCapture(e.pointerId);
      e.preventDefault();
    }
  }

  function onScrollPointerMove(e: PointerEvent) {
    if (startDragPos) {
      const [startMouse, startScroll] = startDragPos;
      const pos = [e.screenX, e.screenY];
      const offset = startMouse.map((p, i) => pos[i] - p);
      const scroll = startScroll.map((o, i) => o - offset[i]);
      const scrollable = document.getElementById(scrollableId)!;
      scrollable.scroll({ left: scroll[0], top: scroll[1] });
      e.preventDefault();
    }
  }
</script>

<div
  id={scrollableId}
  class="grid gap-1 overflow-y-auto"
  style={`grid-template-rows: repeat(${totalRows + 2}, 2.5em); grid-template-columns: repeat(3, max-content) 1fr`}
  onscrollend={(e) => {
    const element = e.target as HTMLElement;
    context.scrollLeft = element.scrollLeft;
    context.scrollTop = element.scrollTop;
  }}
  {@attach (e) => {
    e.scrollLeft = context.scrollLeft;
    e.scrollTop = context.scrollTop;
  }}
>
  <!-- The zoom controls -->
  <div class="col-start-4 row-start-1 z-[100] group sticky top-0" {@attach pan}>
    <div class="flex w-fit h-[2.5em] fixed right-[2em] items-center gap-1">
      <button
        class="btn-icon preset-filled opacity-0 transition-opacity group-hover:opacity-100"
        onclick={() => (context.scale = context.scale * 1.1)}><ZoomIn /></button
      >
      <button
        class="btn-icon preset-filled opacity-0 transition-opacity group-hover:opacity-100"
        onclick={() => (context.scale = context.scale / 1.1)}
        ><ZoomOut /></button
      >
    </div>
  </div>

  <!-- The first three, frozen, columns -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-1 col-end-4 grid left-0 sticky bg-surface-50-950 z-40 border-r"
    style={`grid-row: 1 / span ${totalRows + 2};`}
  >
    <div class="font-bold p-1 bg-teal-800 sticky top-0">Product Epic</div>
    <div class="font-bold p-1 bg-teal-800 sticky top-0">Target Date</div>
    <div class="font-bold p-1 bg-teal-800 sticky top-0">
      Engineering Scenarios
    </div>

    {#each data.epics as epic, epicIndex}
      <div
        class="col-start-1 p-1"
        style={`grid-row: span ${getEpicRowSpan(epic)}; ${getEpicFillStyle(epicIndex)}`}
      >
        {epic.name}
      </div>
      <div
        class="p-1"
        style={`grid-row: span ${getEpicRowSpan(epic)}; ${getEpicFillStyle(epicIndex)}`}
      >
        {epic.targetDate}
      </div>
      {#each epic.scenarios as scenario}
        <div
          class="p-1 col-start-3 group"
          style={`grid-row: span ${scenario.rows.length}; ${getEpicFillStyle(epicIndex)}`}
        >
          {scenario.name}
          {@render scenarioEditor?.(scenario)}
        </div>
      {/each}
    {/each}
  </div>

  <!-- The date strip in row 1, the vertical lines for dates, and the line for today -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 1 / span ${totalRows + 2};`}
    {@attach pan}
  >
    <!-- Vertical lines -->
    <div
      class="col-start-1 relative"
      style={`grid-row: 1 / span ${totalRows + 1};`}
    >
      {#each dates as date}
        <div
          class="absolute border-l border-surface-400-600 h-full z-10 py-5"
          style={`left: ${date.value}px;`}
        >
          &nbsp;
        </div>
      {/each}

      <div
        class="absolute border-l-4 border-teal-300 h-full py-5 border-dashed"
        style={`left: ${dayjs().unix() * context.scale - minX}px;`}
      >
        &nbsp;
      </div>
    </div>

    <!-- Row 1's date labels -->
    <div class="row-start-1 col-start-1 text-white bg-teal-800 z-20  sticky top-0">
      {#each dates as date}
        <div
          class="absolute"
          style={`left: ${date.value}px; transform: translate(-50%,0)`}
        >
          {date.label}
        </div>
      {/each}
    </div>
  </div>

  <!-- Background fill for each scenario - color comes from the epic, height
       determined by number of rows in scenario -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 2 / span ${totalRows + 2};`}
  >
    {#each data.epics as epic, epicIndex}
      {#each epic.scenarios as scenario}
        <div
          style={`${getEpicFillStyle(epicIndex)} grid-row: span ${scenario.rows.length};`}
        >
          &nbsp;
        </div>
      {/each}
    {/each}
  </div>

  <!-- The bars themselves -->
  <div
    class="grid-cols-subgrid grid-rows-subgrid col-start-4 col-end-5 w-full grid"
    style={`grid-row: 2 / span ${totalRows + 2};`}
    {@attach pan}
  >
    {#each data.epics as epic, epicIndex}
      {#each epic.scenarios as scenario}
        {#each scenario.rows as row}
          <div
            class="col-start-1 text-xs relative text-black"
            style={`width: ${maxX - minX}px`}
          >
            {#each row.bars as bar}
              {@const start = convertDate(bar.start) - minX}
              {@const width = convertDate(bar.end) - convertDate(bar.start)}
              <div
                class="absolute content-center text-center h-full z-10"
                style={`left: ${start}px; max-width: ${width}px; width: ${width}px;`}
              >
                <div
                  class="w-full h-[2em] text-nowrap overflow-ellipsis overflow-clip content-center text-center rounded-r-xl group"
                  style={`${getBarFillStyle(bar.state)};`}
                >
                  {#if bar.label}
                    {bar.label}
                  {:else}
                    &nbsp;
                  {/if}
                  {@render barEditor?.(bar)}
                </div>
              </div>
            {/each}
          </div>
        {/each}
      {/each}
    {/each}
  </div>
</div>
