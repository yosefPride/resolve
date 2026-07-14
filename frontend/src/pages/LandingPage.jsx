import Hero from '../components/marketing/Hero';
import FeatureGrid from '../components/marketing/FeatureGrid';
import WorkflowTimeline from '../components/marketing/WorkflowTimeline';
import AudienceCards from '../components/marketing/AudienceCards';

export default function LandingPage() {
  return (
    <>
      <Hero />
      <FeatureGrid />
      <WorkflowTimeline />
      <AudienceCards />
    </>
  );
}
